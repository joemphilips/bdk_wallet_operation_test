use bdk::bitcoin::{Transaction, Amount};
use bdk::bitcoin::hashes::hex::ToHex;
use bdk::bitcoin::util::bip32::{DerivationPath, ExtendedPubKey, Fingerprint};
use bdk::blockchain::Blockchain;
use bdk::blockchain::rpc::Auth;
use bdk::miniscript::psbt::PsbtExt;
use bdk::signer::{InputSigner, SignerWrapper};
use bdk::template::Bip84Public;
use bdk::{
    bitcoin::{secp256k1::Secp256k1, Network},
    blockchain::{ConfigurableBlockchain, RpcBlockchain, RpcConfig},
    keys::{
        DerivableKey,
    },
    signer::{SignerOrdering, TransactionSigner},
    sled,
    wallet::AddressIndex,
    KeychainKind, SignOptions, SyncOptions, Wallet,
};
use electrsd::bitcoind::bitcoincore_rpc::RpcApi;
use std::path::Path;
use std::{error::Error, path::PathBuf, str::FromStr, sync::Arc};

use crate::{bdk_to_electrsd_addr, electrsd_to_bdk_script, bdk_to_electrsd_amt};

pub fn watchonly_wallet_send_all<T: InputSigner + 'static>(
    signer: T,
    xpub: ExtendedPubKey,
    xpub_parent_fingerprint: Fingerprint,
    wallet_name: String,
) -> Result<(), Box<dyn Error>> {
    let datadir = {
        let mut d = PathBuf::from_str("/tmp/")?;
        d.push("watchonly.bdk-example");
        d
    };
    if Path::exists(&datadir) {
        std::fs::remove_dir_all(&datadir)?;
    }

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
        println!("creating db in : {}", datadir.to_str().unwrap());
        let d = sled::open(datadir)?;
        d.open_tree(wallet_name.clone())?
    };

    println!("creating wallet");
    let mut wallet = Wallet::new(
        Bip84Public(xpub.clone(), xpub_parent_fingerprint, KeychainKind::External),
        Some(Bip84Public(
            xpub.clone(),
            xpub_parent_fingerprint,
            KeychainKind::Internal,
        )),
        Network::Regtest,
        database,
    )?;
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
            wallet_name,
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
    println!("sending wallet address {} from bitcoind", bdk_new_addr);
    bitcoind.client.send_to_address(
        &bdk_new_addr,
        bdk_to_electrsd_amt(Amount::from_btc(0.1)?),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    bitcoind.client.generate_to_address(3, &bdk_new_addr)?;
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

    let mut builder = wallet.build_tx();
    builder
        .drain_to(electrsd_to_bdk_script(core_address.script_pubkey()))
        .drain_wallet();

    println!(">> signing psbt");
    let (mut psbt, _) = builder.finish().unwrap();
    println!("{}", psbt.to_string());

    wallet.sign(&mut psbt, SignOptions::default())?;

    let tx: Transaction = psbt.extract_tx();
    println!("Finished creating tx: {}", bdk::bitcoin::consensus::serialize(&tx).to_hex());
    println!("Remaining BDK wallet balance: {}", wallet.get_balance()?);
    blockchain.broadcast(&tx)?;
    println!("Finished broadcasting tx: {}", tx.ntxid());
    Ok(())
}
