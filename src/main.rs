use bdk::{
    bitcoin::{
        secp256k1::Secp256k1,
        util::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey},
        Network,
    },
    descriptor::{DescriptorXKey, Wildcard},
    keys::DerivableKey,
    signer::SignerWrapper,
};
use clap::Parser;
use std::{error::Error, str::FromStr};
use wallet_operation_test::{
    generate_random_ext_privkey, send_bitcoin::wallet_send_tx, watchonly::watchonly_wallet_send_all,
};

#[derive(Debug, Parser)]
#[clap(name = "wallet_operation_test", author, about, version)]
struct Args {
    #[arg(short, long, default_value = "send_from_watchonly")]
    mode: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();
    match args.mode.as_str() {
        "send_bitcoin" => wallet_send_tx(),
        "send_from_watchonly" => {
            let secp = Secp256k1::new();
            let seed = generate_random_ext_privkey()?;
            let master_extkey = seed.clone().into_extended_key()?;
            let path = DerivationPath::from_str("m/84'/0'/0'")?;
            let xprv = master_extkey
                .into_xprv(Network::Regtest)
                .unwrap()
                .derive_priv(&secp, &path)?;
            let xpub: ExtendedPubKey = ExtendedPubKey::from_priv(&secp, &xprv);
            let fingerprint = seed
                .into_extended_key()?
                .into_xprv(Network::Regtest)
                .unwrap()
                .fingerprint(&secp);

            // on-memory InputSigner for testing.
            let dummy_signer = {
                let bip84path = path;
                let signer = DescriptorXKey::<ExtendedPrivKey> {
                    origin: Some((fingerprint, bip84path)),
                    xkey: xprv,
                    derivation_path: DerivationPath::from_str("m/0").unwrap(),
                    wildcard: Wildcard::Unhardened,
                };
                SignerWrapper::<DescriptorXKey<ExtendedPrivKey>>::new(
                    signer,
                    bdk::signer::SignerContext::Segwitv0,
                )
            };

            watchonly_wallet_send_all(
                dummy_signer,
                xpub,
                fingerprint,
                "watchonly_wallet".to_string(),
            )
        }
        _ => panic!("mode must be one of send_bitcoin, send_from_watchonly"),
    }
}
