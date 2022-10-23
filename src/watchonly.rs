use bdk::bitcoin::hashes::hex::ToHex;
use bdk::bitcoin::util::bip32::{ExtendedPubKey, Fingerprint, ExtendedPrivKey};
use bdk::bitcoin::Transaction;
use bdk::blockchain::rpc::Auth;
use bdk::blockchain::Blockchain;
use bdk::signer::InputSigner;
use bdk::template::{Bip84Public};
use bdk::{
    bitcoin::Network,
    blockchain::{ConfigurableBlockchain, RpcBlockchain, RpcConfig},
    signer::SignerOrdering,
    sled,
    wallet::AddressIndex,
    KeychainKind, SignOptions, SyncOptions, Wallet,
};
use electrsd::bitcoind::bitcoincore_rpc::RpcApi;
use std::{error::Error, path::PathBuf, str::FromStr, sync::Arc};

use crate::wallet_backup::WalletBackupData;
use crate::{bdk_to_electrsd_addr, electrsd_to_bdk_script, get_bip84_public_descriptor_templates};

pub fn watchonly_wallet_send_all<T: InputSigner + 'static>(
    signer: T,
    change_signer: T,
    mut wallet_backup: WalletBackupData,
    datadir: PathBuf,
) -> Result<(), Box<dyn Error>> {
    // 0. setup background bitcoind process
    println!(">> Setting up bitcoind");
    let bitcoind = {
        let bitcoind_conf = electrsd::bitcoind::Conf::default();
        let exe = electrsd::bitcoind::downloaded_exe_path()
            .expect("We should always have downloaded path");
        electrsd::bitcoind::BitcoinD::with_conf(exe, &bitcoind_conf).unwrap()
    };

    let core_address = bitcoind.client.get_new_address(None, None)?;
    bitcoind.client.generate_to_address(101, &core_address)?;
    println!(">> bitocoind setup complete");
    println!(
        "Available coins in Core wallet : {}",
        bitcoind.client.get_balance(None, None)?
    );

    // 1. create wallet.
    println!("creating xpub");

    let database = {
        let mut db_path = datadir.clone();
        db_path.push("wallet.db");
        println!("creating db in : {}", db_path.to_str().unwrap());
        let d = sled::open(db_path)?;
        d.open_tree(wallet_backup.get_wallet_name())?
    };

    println!("creating wallet");
    let mut wallet =
      {
        let (d, change_d) = wallet_backup.descriptors[0].clone();
        Wallet::new(
          d,
          change_d,
          Network::Regtest,
          database,
        )?
      };
    println!(">> watch-only wallet created successfully");

    // 2. sync wallet
    let bitcoind = {
        let bitcoind_conf = electrsd::bitcoind::Conf::default();
        let exe = electrsd::bitcoind::downloaded_exe_path()
            .expect("We should always have downloaded path");
        electrsd::bitcoind::BitcoinD::with_conf(exe, &bitcoind_conf).unwrap()
    };
    let bitcoind_auth = Auth::Cookie {
        file: bitcoind.params.cookie_file.clone(),
    };
    let blockchain = {
        let rpc_config = RpcConfig {
            url: bitcoind.params.rpc_socket.to_string(),
            auth: bitcoind_auth,
            network: Network::Regtest,
            wallet_name: wallet_backup.get_wallet_name(),
            sync_params: None,
        };
        RpcBlockchain::from_config(&rpc_config)?
    };
    wallet.sync(&blockchain, SyncOptions::default())?;

    // 3. prepare wallet balance.
    println!(">> BDK wallet setup complete.");
    println!(
        "Available initial coins in BDK wallet : {} sats",
        wallet.get_balance()?
    );
    println!(">> preparing wallet funds");
    let bdk_new_addr = bdk_to_electrsd_addr(wallet.get_address(AddressIndex::New)?.address);
    bitcoind.client.generate_to_address(101, &bdk_new_addr)?;
    wallet.sync(&blockchain, SyncOptions::default())?;
    // check the wallet has spendable balance.
    {
        let balance = wallet.get_balance()?;
        println!("confirmed wallet balance is {}", balance.confirmed);
        assert!(balance.confirmed > 0);
    };

    // 3. add signer and sign
    println!(">> adding signer");

    wallet.add_signer(
        KeychainKind::External,
        SignerOrdering(100),
        Arc::new(signer),
    );
    wallet.add_signer(
      KeychainKind::Internal,
      SignerOrdering(100),
      Arc::new(change_signer)
    );

    let mut builder = wallet.build_tx();

    let core_spk_bdk =
        // needs conversion between crates.
          electrsd_to_bdk_script(core_address.script_pubkey());
    builder.set_recipients(vec![(core_spk_bdk, 500000)]);

    println!(">> signing psbt");
    let (mut psbt, _) = builder.finish().unwrap();
    println!("{}", psbt.to_string());

    wallet.sign(&mut psbt, SignOptions::default())?;

    let tx: Transaction = psbt.extract_tx();
    println!(
        "Finished creating tx: {}",
        bdk::bitcoin::consensus::serialize(&tx).to_hex()
    );
    blockchain.broadcast(&tx)?;
    println!("Finished broadcasting tx: {}", tx.ntxid());
    wallet.sync(&blockchain, SyncOptions::default())?;
    println!("Remaining BDK wallet balance: {}", wallet.get_balance()?);
    Ok(())
}
