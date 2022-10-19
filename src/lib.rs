use std::error::Error;
use std::str::FromStr;

use bdk::{bitcoin::hashes::hex::{FromHex, ToHex}, keys::{bip39::{Mnemonic, Language, WordCount}, GeneratedKey, GeneratableKey}, miniscript::Segwitv0};

pub mod send_bitcoin;
pub mod watchonly;

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


pub fn generate_random_ext_privkey() -> Result<(GeneratedKey<Mnemonic, Segwitv0>, Option<String>), Box<dyn Error>> {
    let password = Some("Random password".to_string());
    let mnemonic = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
    Ok((mnemonic, password))
}