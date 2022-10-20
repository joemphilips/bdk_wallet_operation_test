use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::{Amount, Network};
use bdk::blockchain::rpc::Auth;
use bdk::blockchain::{Blockchain, ConfigurableBlockchain, RpcBlockchain, RpcConfig};
use bdk::template::{Bip84, DescriptorTemplate};
use bdk::wallet::{wallet_name_from_descriptor, AddressIndex};
use bdk::{sled, KeychainKind, SignOptions, SyncOptions, Wallet};
use electrsd::bitcoind::bitcoincore_rpc::RpcApi;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use crate::{
    bdk_to_electrsd_addr, bdk_to_electrsd_amt, electrsd_to_bdk_script, generate_random_ext_privkey,
};

pub fn wallet_send_tx() -> Result<(), Box<dyn Error>> {
    // 0. setup background bitcoind process
    println!(">> Setting up bitcoind");
    let bitcoind = {
        let bitcoind_conf = electrsd::bitcoind::Conf::default();
        let exe = electrsd::bitcoind::downloaded_exe_path()
            .expect("We should always have downloaded path");
        electrsd::bitcoind::BitcoinD::with_conf(exe, &bitcoind_conf).unwrap()
    };
    let bitcoind_auth = Auth::Cookie {
        file: bitcoind.params.cookie_file.clone(),
    };
    let core_address = bitcoind.client.get_new_address(None, None)?;
    bitcoind.client.generate_to_address(101, &core_address)?;
    println!(">> bitocoind setup complete");
    println!(
        "Available coins in Core wallet : {}",
        bitcoind.client.get_balance(None, None)?
    );
    let secp = &Secp256k1::new();

    // 1. instantiate the wallet.
    let xprv = generate_random_ext_privkey()?;
    let descriptor = Bip84(xprv.clone(), KeychainKind::External);
    let change = Bip84(xprv.clone(), KeychainKind::Internal);
    println!("*************************************\n");
    println!("* These information are important for recovering your funds! please take a backup *");
    println!("* wallet seedphrase: \"{}\"", xprv.clone().0.into_key());
    if let Some(pass) = xprv.clone().1 {
        println!("* password: \"{}\"", pass);
    }
    println!(
        "* descriptor: \"{}\"",
        descriptor.build(Network::Regtest)?.0
    );
    println!(
        "* change descriptor: \"{}\"",
        change.build(Network::Regtest)?.0
    );
    println!("*************************************\n");

    let wallet_name = wallet_name_from_descriptor(
        Bip84(xprv.clone(), KeychainKind::External),
        Some(Bip84(xprv.clone(), KeychainKind::Internal)),
        Network::Regtest,
        &secp,
    )?;
    let database = {
        let datadir = {
            let mut d = PathBuf::from_str("/tmp/")?;
            d.push(".bdk-example");
            d
        };
        let d = sled::open(datadir)?;
        d.open_tree(wallet_name.clone())?
    };
    let wallet = Wallet::new(
        Bip84(xprv.clone(), KeychainKind::External),
        Some(Bip84(xprv.clone(), KeychainKind::Internal)),
        Network::Regtest,
        database,
    )?;

    // 2. sync wallet

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
    // check the wallet has spendable balance.
    {
        let balance = wallet.get_balance()?;
        assert!(balance.confirmed == 0);
    };
    println!(">> BDK wallet setup complete.");
    println!(
        "Available initial coins in BDK wallet : {} sats",
        wallet.get_balance()?
    );
    println!("\n>> Sending coins: Core --> BDK, 10 BTC");

    // 3. prepare wallet balance.
    let bdk_new_addr = bdk_to_electrsd_addr(wallet.get_address(AddressIndex::New)?.address);
    bitcoind.client.send_to_address(
        &bdk_new_addr,
        bdk_to_electrsd_amt(Amount::from_btc(10.0)?),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    bitcoind.client.generate_to_address(3, &bdk_new_addr)?;
    wallet.sync(&blockchain, SyncOptions::default())?;

    println!(">> Received coins in BDK wallet");
    println!(
        "Available balance in BDK wallet: {} sats",
        wallet.get_balance()?
    );

    // 4. build and send tx from the wallet
    println!("\n>> Sending coins: BDK --> Core, 5 BTC");
    let mut txb = Wallet::build_tx(&wallet);
    let core_spk_bdk =
        // needs conversion between crates.
          electrsd_to_bdk_script(core_address.script_pubkey());
    txb.set_recipients(vec![(core_spk_bdk, 500000)]);
    let (mut psbt, _tx_details) = txb.finish()?;

    {
        let sign_options = SignOptions {
            assume_height: None,
            ..Default::default()
        };
        wallet.sign(&mut psbt, sign_options)?;
    };
    let tx = psbt.extract_tx();
    blockchain.broadcast(&tx)?;
    println!("Finished broadcasting tx: {}", tx.ntxid());
    println!("Remaining BDK wallet balance: {}", wallet.get_balance()?);
    Ok(())
}
