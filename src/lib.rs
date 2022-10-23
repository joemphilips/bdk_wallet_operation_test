use std::error::Error;
use std::str::FromStr;

use bdk::{
    bitcoin::{hashes::hex::{FromHex, ToHex}, util::bip32::{ExtendedPrivKey, Fingerprint, ExtendedPubKey}, secp256k1::Secp256k1},
    keys::{
        bip39::{Language, Mnemonic, WordCount},
        GeneratableKey, GeneratedKey, DerivableKey,
    },
    miniscript::Segwitv0, descriptor::ExtendedDescriptor, template::{Bip84Public, Bip84}, KeychainKind, Wallet,
};

pub mod send_bitcoin;
pub mod watchonly;
pub mod wallet_backup;

pub(crate) fn bdk_to_electrsd_addr(
    bdk: bdk::bitcoin::Address,
) -> electrsd::bitcoind::bitcoincore_rpc::bitcoin::Address {
    electrsd::bitcoind::bitcoincore_rpc::bitcoin::Address::from_str(bdk.to_string().as_str())
        .unwrap()
}
pub(crate) fn bdk_to_electrsd_amt(
    bdk: bdk::bitcoin::Amount,
) -> electrsd::bitcoind::bitcoincore_rpc::bitcoin::Amount {
    electrsd::bitcoind::bitcoincore_rpc::bitcoin::Amount::from_sat(bdk.as_sat())
}
pub(crate) fn electrsd_to_bdk_script(
    e: electrsd::bitcoind::bitcoincore_rpc::bitcoin::Script,
) -> bdk::bitcoin::Script {
    let c = e.to_hex();
    bdk::bitcoin::Script::from_hex(&c).unwrap()
}

pub fn generate_random_ext_privkey(
) -> Result<(GeneratedKey<Mnemonic, Segwitv0>, Option<String>), Box<dyn Error>> {
    let password = Some("Random password".to_string());
    let mnemonic = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
    Ok((mnemonic, password))
}

pub fn get_bip84_templates<K: DerivableKey<Segwitv0> + Clone>(xprv: &K) -> (Bip84<K>, Bip84<K>) {
  (Bip84(xprv.clone(), KeychainKind::External), Bip84(xprv.clone(), KeychainKind::Internal))
}

pub fn get_bip84_public_descriptor_templates(
  xpub: ExtendedPubKey,
  master_fingerprint: Fingerprint
) -> (Bip84Public<ExtendedPubKey>, Bip84Public<ExtendedPubKey>) {
    let d = Bip84Public(
        xpub.clone(),
        master_fingerprint,
        KeychainKind::External,
    );
    let change_d = Bip84Public(
        xpub.clone(),
        master_fingerprint,
        KeychainKind::Internal,
    );
    (d, change_d)
}


pub fn get_wallet_name<K: DerivableKey<Segwitv0> + Clone>(master_xprv: &K) -> Result<String, bdk::Error> {
    let secp = Secp256k1::new();
    let (d, c) = get_bip84_templates(master_xprv);
    bdk::wallet::wallet_name_from_descriptor(
        d,
        Some(c),
        bdk::bitcoin::Network::Regtest,
        &secp,
    )
}
