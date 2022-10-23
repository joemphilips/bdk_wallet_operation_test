use std::{error::Error, str::FromStr};
use bdk::{bitcoin::{util::bip32::{ExtendedPrivKey, DerivationPath, ExtendedPubKey, Fingerprint}, secp256k1::{Secp256k1, All}, Network}, descriptor::{ExtendedDescriptor, IntoWalletDescriptor, DescriptorXKey, Wildcard}, keys::DerivableKey, template::DescriptorTemplate, signer::{InputSigner, SignerWrapper}};

use crate::{generate_random_ext_privkey, get_wallet_name, get_bip84_public_descriptor_templates};


/// Pair of 1. wallet descriptor for receiving funds. 2. wallet descriptor for the change (optional).
type DescriptorPair = (ExtendedDescriptor, Option<ExtendedDescriptor>);
pub const BIP84_HARDENED_PATH: &'static str = "m/84'/0'/0'";

/// Serializable information for recovering a wallet in arbitrary descriptor-based wallet.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WalletBackupData {
  pub wif: ExtendedPrivKey,
  pub descriptors: Vec<DescriptorPair>,

  #[serde(skip_serializing_if="Option::is_none")]
  wallet_name: Option<String>,
}

impl WalletBackupData {
    fn verify_fingerprint(&self) -> Result<(), Box<dyn Error>> {
      for d in self.descriptors {
        if let Some(f) = d.0. {

        }
      }
      Ok(())
    }

    pub fn generate_bip84(network: Network) -> Result<Self, Box<dyn Error>> {
        let secp = Secp256k1::new();
        let seed = generate_random_ext_privkey()?;
        let path = DerivationPath::from_str(BIP84_HARDENED_PATH)?;
        let wif = seed.clone().into_extended_key()?
            .into_xprv(network)
            .unwrap();
        let xprv = wif.derive_priv(&secp, &path)?;
        let xpub: ExtendedPubKey = ExtendedPubKey::from_priv(&secp, &xprv);
        let (desc, change_desc) = get_bip84_public_descriptor_templates(xpub, wif.fingerprint(&secp));
        Ok (WalletBackupData {
            wif,
            descriptors: vec![
                (desc.build(network)?.0, Some(change_desc.build(network)?.0))
            ],
            wallet_name: Some(get_wallet_name(&seed)?),
        })
    }

    pub(crate) fn get_dummy_signers(&self) -> impl InputSigner {
      let signer = DescriptorXKey::<ExtendedPrivKey> {
          origin: Some((self.get_fingerprint(), DerivationPath::from_str(BIP84_HARDENED_PATH).unwrap())),
          xkey: xprv,
          derivation_path: DerivationPath::from_str("m/0").unwrap(),
          wildcard: Wildcard::Unhardened,
      };
      SignerWrapper::<DescriptorXKey<ExtendedPrivKey>>::new(
          signer,
          bdk::signer::SignerContext::Segwitv0,
      )
    }

    pub fn get_wallet_name(&mut self) -> String {
      self
        .wallet_name
        .get_or_insert_with(|| get_wallet_name(&self.wif).unwrap())
        .to_string()
    }


    pub fn get_fingerprint(&self) -> Fingerprint {
      self.wif.fingerprint(&Secp256k1::new())
    }
}
