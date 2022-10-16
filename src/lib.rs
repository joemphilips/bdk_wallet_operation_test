use std::str::FromStr;

use bdk::bitcoin::hashes::hex::{ToHex, FromHex};


pub mod send_bitcoin;
pub mod watchonly;

pub(crate) fn bdk_to_electrsd_addr(bdk: bdk::bitcoin::Address) -> electrsd::bitcoind::bitcoincore_rpc::bitcoin::Address {
    electrsd::bitcoind::bitcoincore_rpc::bitcoin::Address::from_str(bdk.to_string().as_str()).unwrap()
}
pub(crate) fn bdk_to_electrsd_amt(bdk: bdk::bitcoin::Amount) -> electrsd::bitcoind::bitcoincore_rpc::bitcoin::Amount {
    electrsd::bitcoind::bitcoincore_rpc::bitcoin::Amount::from_sat(bdk.as_sat())
}
pub(crate) fn electrsd_to_bdk_script(e: electrsd::bitcoind::bitcoincore_rpc::bitcoin::Script) ->  bdk::bitcoin::Script  {
    let c = e.to_hex();
    bdk::bitcoin::Script::from_hex(&c).unwrap()
}
