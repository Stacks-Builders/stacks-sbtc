use clap::Parser;
use hashbrown::HashMap;
use p256k1::ecdsa;
use serde::Deserialize;
use std::fs;
use toml;
use wsts::Scalar;

use crate::util::parse_public_key;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    #[error("Invalid Public Key: {0}")]
    InvalidPublicKey(String),
    #[error("Invalid Private Key: {0}")]
    InvalidPrivateKey(String),
    #[error("Invalid Key ID. Must specify Key IDs greater than 0.")]
    InvalidKeyID,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Config file path
    #[arg(short, long)]
    pub config: String,

    /// Start a signing round
    #[arg(short, long)]
    pub start: bool,

    /// ID associated with signer
    #[arg(short, long)]
    pub id: u32,
}

#[derive(Clone, Deserialize, Default, Debug)]
struct RawSignerKeys {
    pub public_key: String,
    pub key_ids: Vec<u32>,
}

#[derive(Clone, Deserialize, Default, Debug)]
pub struct RawConfig {
    pub http_relay_url: String,
    pub keys_threshold: u32,
    pub frost_state_file: String,
    pub network_private_key: String,
    signers: Vec<RawSignerKeys>,
    coordinator_public_key: String,
}

impl RawConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<RawConfig, Error> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn signer_keys(&self) -> Result<SignerKeys, Error> {
        let mut signer_keys = SignerKeys::default();
        for (i, s) in self.signers.iter().enumerate() {
            let signer_public_key = parse_public_key(&s.public_key).map_err(|_| {
                Error::InvalidPublicKey(format!(
                    "Failed to parse signers from config. {}",
                    s.public_key
                ))
            })?;
            for key_id in &s.key_ids {
                //We do not allow a key id of 0.
                if *key_id == 0 {
                    return Err(Error::InvalidKeyID);
                }
                signer_keys.key_ids.insert(*key_id, signer_public_key);
            }
            //We start our signer and key IDs from 1 hence the + 1;
            let signer_key = u32::try_from(i).unwrap() + 1;
            signer_keys.signers.insert(signer_key, signer_public_key);
        }
        Ok(signer_keys)
    }

    pub fn coordinator_public_key(&self) -> Result<ecdsa::PublicKey, Error> {
        parse_public_key(&self.coordinator_public_key).map_err(|_| {
            Error::InvalidPublicKey(format!(
                "Failed to parse coordinator_public_key from config. {}",
                self.coordinator_public_key
            ))
        })
    }

    pub fn network_private_key(&self) -> Result<Scalar, Error> {
        let network_private_key =
            Scalar::try_from(self.network_private_key.as_str()).map_err(|_| {
                Error::InvalidPrivateKey(format!(
                    "Failed to parse network_private_key from config. {}",
                    self.network_private_key.clone()
                ))
            })?;
        Ok(network_private_key)
    }
}

#[derive(Default, Clone)]
pub struct SignerKeys {
    pub signers: HashMap<u32, ecdsa::PublicKey>,
    pub key_ids: HashMap<u32, ecdsa::PublicKey>,
}

#[derive(Clone)]
pub struct Config {
    pub http_relay_url: String,
    pub keys_threshold: u32,
    pub frost_state_file: String,
    pub network_private_key: Scalar,
    pub signer_keys: SignerKeys,
    pub coordinator_public_key: ecdsa::PublicKey,
    pub total_signers: u32,
    pub total_keys: u32,
}

impl Config {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Config, Error> {
        let raw_config = RawConfig::from_path(path)?;
        let signer_keys = raw_config.signer_keys()?;
        let total_signers = signer_keys.signers.len().try_into().unwrap();
        let total_keys = signer_keys.key_ids.len().try_into().unwrap();
        let coordinator_public_key = raw_config.coordinator_public_key()?;
        let network_private_key = raw_config.network_private_key()?;
        Ok(Self {
            http_relay_url: raw_config.http_relay_url,
            keys_threshold: raw_config.keys_threshold,
            frost_state_file: raw_config.frost_state_file,
            network_private_key,
            total_signers,
            total_keys,
            signer_keys,
            coordinator_public_key,
        })
    }
}

#[cfg(test)]
mod test {
    use super::{Error, RawConfig, RawSignerKeys};
    #[test]
    fn coordinator_public_key_test() {
        let mut config = RawConfig::default();
        // Should fail with an empty public key
        assert!(matches!(
            config.coordinator_public_key(),
            Err(Error::InvalidPublicKey(_))
        ));
        // Should fail with an invalid public key
        config.coordinator_public_key = "Invalid Public Key".to_string();
        assert!(matches!(
            config.coordinator_public_key(),
            Err(Error::InvalidPublicKey(_))
        ));
        // Should succeed with a valid public key
        config.coordinator_public_key = "22Rm48xUdpuTuva5gz9S7yDaaw9f8sjMcPSTHYVzPLNcj".to_string();
        assert!(config.coordinator_public_key().is_ok());
    }

    #[test]
    fn signers_test() {
        let mut config = RawConfig::default();
        let public_key = "22Rm48xUdpuTuva5gz9S7yDaaw9f8sjMcPSTHYVzPLNcj".to_string();
        // Should succeed with an empty vector
        let signers = config.signer_keys().unwrap();
        assert!(signers.key_ids.is_empty());
        assert!(signers.signers.is_empty());

        // Should fail with an empty public key
        let raw_signer_keys = RawSignerKeys {
            key_ids: vec![1, 2],
            public_key: "".to_string(),
        };
        config.signers = vec![raw_signer_keys];
        assert!(matches!(
            config.signer_keys(),
            Err(Error::InvalidPublicKey(_))
        ));

        // Should fail with an invalid public key
        let raw_signer_keys = RawSignerKeys {
            key_ids: vec![1, 2],
            public_key: "Invalid public key".to_string(),
        };
        config.signers = vec![raw_signer_keys];
        assert!(matches!(
            config.signer_keys(),
            Err(Error::InvalidPublicKey(_))
        ));

        // Should fail with an invalid key ID
        let raw_signer_keys = RawSignerKeys {
            key_ids: vec![0, 1],
            public_key: public_key.clone(),
        };
        config.signers = vec![raw_signer_keys];
        assert!(matches!(config.signer_keys(), Err(Error::InvalidKeyID)));

        // Should succeed with a valid public keys
        let raw_signer_keys1 = RawSignerKeys {
            key_ids: vec![1, 2],
            public_key: public_key.clone(),
        };
        let raw_signer_keys2 = RawSignerKeys {
            key_ids: vec![3, 4],
            public_key,
        };
        config.signers = vec![raw_signer_keys1, raw_signer_keys2];
        let signer_keys = config.signer_keys().unwrap();
        assert_eq!(signer_keys.signers.len(), 2);
        assert_eq!(signer_keys.key_ids.len(), 4);
    }
}
