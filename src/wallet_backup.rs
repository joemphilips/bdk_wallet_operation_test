use bdk::{
    bitcoin::{
        secp256k1::{All, Secp256k1},
        util::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey, Fingerprint},
        Network,
    },
    descriptor::ExtendedDescriptor,
    keys::DerivableKey,
    miniscript::{Descriptor, DescriptorPublicKey, ForEachKey},
    template::DescriptorTemplate,
};
use std::{error::Error, str::FromStr};

use crate::{generate_random_ext_privkey, get_bip84_public_descriptor_templates, get_wallet_name};

/// Pair of 1. wallet descriptor for receiving funds. 2. wallet descriptor for the change (optional).
type DescriptorPair = (ExtendedDescriptor, Option<ExtendedDescriptor>);
pub const BIP84_HARDENED_PATH: &'static str = "m/84'/0'/0'";

/// Serializable information for recovering a wallet in arbitrary descriptor-based wallet.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WalletBackupData {
    pub wif: ExtendedPrivKey,
    pub descriptors: Vec<DescriptorPair>,

    #[serde(skip_serializing_if = "Option::is_none")]
    wallet_name: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WalletBackupError {
    /// We could not find matching master fingerprint in the descriptor.
    NoOriginFingerprint,

    UnsupportedDescriptorType,
}

impl std::fmt::Display for WalletBackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedDescriptorType => {
                write!(f, "The wallet only supports wpkh descriptor")
            }
            Self::NoOriginFingerprint => write!(f, "origin fingerprint not found on descriptor"),
        }
    }
}

impl Error for WalletBackupError {}

impl WalletBackupData {
    /// Verify if we can handle the deserialized backup data.
    pub fn verify(&self, secp: &Secp256k1<All>) -> Result<(), WalletBackupError> {
        let fingerprint = self.get_fingerprint(&secp);
        let verify = |dk: &Descriptor<_>| match dk {
            Descriptor::<DescriptorPublicKey>::Wpkh(k) => {
                let same_fingerprint_found =
                    k.for_any_key(|pk| pk.as_key().master_fingerprint() == fingerprint);
                if !same_fingerprint_found {
                    Err(WalletBackupError::NoOriginFingerprint)
                } else {
                    Ok(())
                }
            }
            _ => Err(WalletBackupError::UnsupportedDescriptorType),
        };
        for (d, c) in &self.descriptors {
            match verify(d).and_then(|()| c.as_ref().map_or(Ok(()), verify)) {
                Ok(()) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn generate_bip84(network: Network) -> Result<Self, Box<dyn Error>> {
        let secp = Secp256k1::new();
        let seed = generate_random_ext_privkey()?;
        let path = DerivationPath::from_str(BIP84_HARDENED_PATH)?;
        let wif = seed
            .clone()
            .into_extended_key()?
            .into_xprv(network)
            .unwrap();
        let xprv = wif.derive_priv(&secp, &path)?;
        let xpub: ExtendedPubKey = ExtendedPubKey::from_priv(&secp, &xprv);
        let (desc, change_desc) =
            get_bip84_public_descriptor_templates(xpub, wif.fingerprint(&secp));
        Ok(WalletBackupData {
            wif,
            descriptors: vec![(desc.build(network)?.0, Some(change_desc.build(network)?.0))],
            wallet_name: Some(get_wallet_name(&seed)?),
        })
    }

    pub fn get_wallet_name(&mut self) -> String {
        self.wallet_name
            .get_or_insert_with(|| get_wallet_name(&self.wif).unwrap())
            .to_string()
    }

    pub fn get_fingerprint(&self, ctx: &Secp256k1<All>) -> Fingerprint {
        self.wif.fingerprint(ctx)
    }
}
