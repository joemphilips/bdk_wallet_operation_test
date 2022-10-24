use bdk::{
    bitcoin::{
        secp256k1::Secp256k1,
        util::bip32::{DerivationPath, ExtendedPrivKey},
        Network,
    },
    descriptor::{DescriptorXKey, Wildcard},
    signer::SignerWrapper,
};
use clap::Parser;
use std::{error::Error, path::PathBuf, str::FromStr};
use wallet_operation_test::{
    send_bitcoin::wallet_send_tx,
    wallet_backup::{WalletBackupData, BIP84_HARDENED_PATH},
    watchonly::watchonly_wallet_send_example,
};

#[derive(Debug, Parser)]
#[clap(name = "wallet_operation_test", author, about, version)]
struct Args {
    #[arg(short, long, default_value = "send_from_watchonly")]
    mode: String,

    #[arg(short, long, default_value = "/tmp")]
    datadir: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Args::parse();
    match args.mode.as_str() {
        "send_bitcoin" => wallet_send_tx(),
        "send_from_watchonly" => {
            let datadir = PathBuf::from_str(args.datadir.clone().as_str())?;

            let backup: WalletBackupData = {
                let backup_path = {
                    let mut w = datadir.clone();
                    w.push("wallet.bck");
                    w
                };
                if std::path::Path::exists(backup_path.as_path()) {
                    let json = std::fs::read_to_string(&backup_path).expect(
                        format!(
                            "Failed to read wallet backup from {}",
                            backup_path.to_str().unwrap()
                        )
                        .as_str(),
                    );
                    serde_json::from_str(&json).expect("failed to read wallet backup as a json")
                } else {
                    let backup = WalletBackupData::generate_bip84(Network::Regtest)?;
                    let wallet_backup_json = serde_json::to_string(&backup)?;
                    println!(
                        "No wallet backup file found. Writing new wallet backup:\n{}\ninto: {}",
                        wallet_backup_json,
                        backup_path.to_str().unwrap()
                    );
                    std::fs::write(backup_path, wallet_backup_json)?;
                    backup
                }
            };
            let secp = Secp256k1::new();
            backup.verify(&secp)?;

            let fingerprint = backup.get_fingerprint(&secp);
            let path = DerivationPath::from_str(BIP84_HARDENED_PATH)?;
            // on-memory InputSigner for testing.
            let dummy_signer = {
                let signer = DescriptorXKey::<ExtendedPrivKey> {
                    origin: Some((fingerprint, path.clone())),
                    xkey: backup.wif.derive_priv(&secp, &path)?,
                    derivation_path: DerivationPath::from_str("m/0").unwrap(),
                    wildcard: Wildcard::Unhardened,
                };
                SignerWrapper::<DescriptorXKey<ExtendedPrivKey>>::new(
                    signer,
                    bdk::signer::SignerContext::Segwitv0,
                )
            };

            let dummy_change_signer = {
                let signer = DescriptorXKey::<ExtendedPrivKey> {
                    origin: Some((fingerprint, path.clone())),
                    xkey: backup.wif.derive_priv(&secp, &path)?,
                    derivation_path: DerivationPath::from_str("m/1").unwrap(),
                    wildcard: Wildcard::Unhardened,
                };
                SignerWrapper::<DescriptorXKey<ExtendedPrivKey>>::new(
                    signer,
                    bdk::signer::SignerContext::Segwitv0,
                )
            };

            watchonly_wallet_send_example(dummy_signer, dummy_change_signer, backup, datadir)
        }
        _ => panic!("mode must be one of send_bitcoin, send_from_watchonly"),
    }
}
