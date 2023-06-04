use blockstack_lib::{
    address::AddressHashMode,
    burnchains::Address,
    chainstate::stacks::TransactionVersion,
    types::chainstate::{StacksAddress, StacksPrivateKey, StacksPublicKey},
    vm::ContractName,
};

use crate::util::address_version;

/// Errors associated with reading the Config file
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Toml Error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Invalid config file. {0}")]
    InvalidConfig(String),
    #[error("Invalid sbtc_contract. {0}")]
    InvalidContract(String),
    #[error("Failed to parse stacks_private_key: {0}")]
    InvalidPrivateKey(String),
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
}

#[derive(serde::Deserialize, Default)]
pub struct RawConfig {
    pub sbtc_contract: String,
    pub stacks_private_key: String,
    pub stacks_node_rpc_url: String,
    pub bitcoin_node_rpc_url: String,
    pub frost_dkg_round_id: u64,
    pub signer_config_path: Option<String>,
    pub start_block_height: Option<u64>,
    pub rusqlite_path: Option<String>,
    /// The network version we are using ('mainnet' or 'testnet'). Default: 'mainnet'
    pub network: Option<Network>,
    /// The transaction fee in Satoshis used to broadcast transactions to the stacks node
    pub transaction_fee: u64,
    /// Frost specific config options. Must be specified if signer_config_path is not used
    pub http_relay_url: Option<String>,
    pub frost_state_file: Option<String>,
    pub network_private_key: Option<String>,
}

impl RawConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let config: RawConfig = toml::from_str(&std::fs::read_to_string(path)?)?;
        Ok(config)
    }

    pub fn parse_contract(&self) -> Result<(ContractName, StacksAddress), Error> {
        let mut split = self.sbtc_contract.split('.');
        let contract_address = split
            .next()
            .ok_or(Error::InvalidContract("Missing address".to_string()))?;
        let contract_name = split
            .next()
            .ok_or(Error::InvalidContract("Missing name.".to_string()))?
            .to_owned();

        let contract_address = StacksAddress::from_string(contract_address)
            .ok_or(Error::InvalidContract("Bad contract address.".to_string()))?;
        let contract_name = ContractName::try_from(contract_name)
            .map_err(|e| Error::InvalidContract(format!("Bad contract name: {}.", e)))?;
        Ok((contract_name, contract_address))
    }

    pub fn parse_stacks_private_key(&self) -> Result<(StacksPrivateKey, StacksAddress), Error> {
        let sender_key = StacksPrivateKey::from_hex(&self.stacks_private_key)
            .map_err(|e| Error::InvalidPrivateKey(e.to_string()))?;

        let pk = StacksPublicKey::from_private(&sender_key);

        let address = StacksAddress::from_public_keys(
            address_version(&self.parse_version().0),
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![pk],
        )
        .ok_or(Error::InvalidPrivateKey(
            "Failed to generate stacks address from private key.".to_string(),
        ))?;

        Ok((sender_key, address))
    }

    pub fn parse_version(&self) -> (TransactionVersion, bitcoin::Network) {
        // Determine what network we are running on
        match self.network.as_ref().unwrap_or(&Network::Mainnet) {
            Network::Mainnet => (TransactionVersion::Mainnet, bitcoin::Network::Bitcoin),
            Network::Testnet => (TransactionVersion::Testnet, bitcoin::Network::Testnet),
        }
    }
}

pub struct Config {
    pub contract_name: ContractName,
    pub contract_address: StacksAddress,
    pub stacks_private_key: StacksPrivateKey,
    pub stacks_address: StacksAddress,
    pub stacks_node_rpc_url: String,
    pub bitcoin_node_rpc_url: String,
    pub frost_dkg_round_id: u64,
    pub signer_config_path: Option<String>,
    pub start_block_height: Option<u64>,
    pub rusqlite_path: Option<String>,
    pub bitcoin_network: bitcoin::Network,
    pub stacks_version: TransactionVersion,
    /// The transaction fee in Satoshis used to broadcast transactions to the stacks node
    pub transaction_fee: u64,
    /// Frost specific config options. Must be specified if signer_config_path is not used
    pub http_relay_url: Option<String>,
    pub frost_state_file: Option<String>,
    pub network_private_key: Option<String>,
}

impl TryFrom<RawConfig> for Config {
    type Error = Error;
    fn try_from(config: RawConfig) -> Result<Self, Error> {
        if config.signer_config_path.is_none() {
            if config.http_relay_url.is_none() {
                return Err(Error::InvalidConfig(
                    "Must specify http_relay_url when no signer_config_path specified.".to_string(),
                ));
            }
            if config.frost_state_file.is_none() {
                return Err(Error::InvalidConfig(
                    "Must specify frost_state_file when no signer_config_path specified."
                        .to_string(),
                ));
            }
            if config.network_private_key.is_none() {
                return Err(Error::InvalidConfig(
                    "Must specify network_private_key when no signer config_path specified."
                        .to_string(),
                ));
            }
        }
        let (contract_name, contract_address) = config.parse_contract()?;
        let (stacks_version, bitcoin_network) = config.parse_version();
        let (stacks_private_key, stacks_address) = config.parse_stacks_private_key()?;

        Ok(Self {
            contract_name,
            contract_address,
            stacks_private_key,
            stacks_address,
            stacks_node_rpc_url: config.stacks_node_rpc_url,
            bitcoin_node_rpc_url: config.bitcoin_node_rpc_url,
            frost_dkg_round_id: config.frost_dkg_round_id,
            signer_config_path: config.signer_config_path,
            start_block_height: config.start_block_height,
            rusqlite_path: config.rusqlite_path,
            bitcoin_network,
            stacks_version,
            transaction_fee: config.transaction_fee,
            http_relay_url: config.http_relay_url,
            frost_state_file: config.frost_state_file,
            network_private_key: config.network_private_key,
        })
    }
}

impl Config {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let raw_config: RawConfig = toml::from_str(&std::fs::read_to_string(path)?)?;
        let config = Config::try_from(raw_config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test::PRIVATE_KEY_HEX;

    use super::*;
    use bitcoin::Network as BitcoinNetwork;
    use blockstack_lib::chainstate::stacks::TransactionVersion;
    use std::io::Write;
    use tempdir::TempDir;

    fn write_new_config(
        sbtc_contract: String,
        stacks_private_key: String,
        signer_config_file: Option<String>,
        http_relay_url: Option<String>,
        network_private_key: Option<String>,
        frost_state_file: Option<String>,
    ) -> Result<Config, Error> {
        let dir = TempDir::new("").unwrap();
        let file_path = dir.path().join("coordinator.toml");
        let mut coord_file = std::fs::File::create(&file_path).unwrap();
        let mut coord_contents = format!(
            r#"
sbtc_contract = "{sbtc_contract}"
stacks_private_key = "{stacks_private_key}"
stacks_node_rpc_url = "http://localhost:20443"
bitcoin_node_rpc_url = "http://localhost:9776"
frost_dkg_round_id = 0
transaction_fee = 2000
"#
        );

        if let Some(http_relay_url) = http_relay_url {
            coord_contents = format!("{coord_contents}\nhttp_relay_url = \"{http_relay_url}\"");
        }
        if let Some(network_private_key) = network_private_key {
            coord_contents =
                format!("{coord_contents}\nnetwork_private_key = \"{network_private_key}\"");
        }
        if let Some(frost_state_file) = frost_state_file {
            coord_contents = format!("{coord_contents}\nfrost_state_file = \"{frost_state_file}\"");
        }
        if let Some(signer_config_file) = signer_config_file {
            coord_contents =
                format!("{coord_contents}\nsigner_config_path = \"{signer_config_file}\"",);
        }
        coord_file.write_all(coord_contents.as_bytes()).unwrap();
        Config::from_path(file_path)
    }

    #[test]
    fn config_from_raw() {
        let sbtc_contract_address = "ST3N4AJFZZYC4BK99H53XP8KDGXFGQ2PRSPNET8TN";
        let sbtc_contract_name = "sbtc-alpha";
        let stacks_private_key = PRIVATE_KEY_HEX;

        // Test config with signer_config_file
        let config = write_new_config(
            format!("{sbtc_contract_address}.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            Some(String::new()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(config.bitcoin_network, BitcoinNetwork::Bitcoin);
        assert_eq!(config.stacks_version, TransactionVersion::Mainnet);
        assert_eq!(config.contract_name.to_string(), sbtc_contract_name);
        assert_eq!(config.contract_address.to_string(), sbtc_contract_address);
        assert_eq!(config.stacks_private_key.to_hex(), stacks_private_key);

        // Test config with no signer_config_file
        let config = write_new_config(
            format!("{sbtc_contract_address}.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        )
        .unwrap();
        assert_eq!(config.bitcoin_network, BitcoinNetwork::Bitcoin);
        assert_eq!(config.stacks_version, TransactionVersion::Mainnet);
        assert_eq!(config.contract_name.to_string(), sbtc_contract_name);
        assert_eq!(config.contract_address.to_string(), sbtc_contract_address);
        assert_eq!(config.stacks_private_key.to_hex(), stacks_private_key);

        // Test config with no signer_config_file or http_relay_url
        let config = write_new_config(
            format!("{sbtc_contract_address}.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            None,
            None,
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidConfig(_))));

        // Test config with no signer_config_file or network_private_key
        let config = write_new_config(
            format!("{sbtc_contract_address}.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            None,
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidConfig(_))));

        // Test config with no signer_config_file or frost_state_file
        let config = write_new_config(
            format!("{sbtc_contract_address}.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            None,
        );
        assert!(matches!(config, Err(Error::InvalidConfig(_))));

        // Test config with invalid contract name
        let config = write_new_config(
            format!("garbage.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidContract(_))));

        // Test config with missing "."
        let config = write_new_config(
            format!("{sbtc_contract_address}"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidContract(_))));

        // Test config with no contract name
        let config = write_new_config(
            format!("{sbtc_contract_address}."),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidContract(_))));

        // Test config with garbage contract name
        let config = write_new_config(
            format!("{sbtc_contract_address}.12"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidContract(_))));

        // Test config with no garbage contract address name
        let config = write_new_config(
            format!("garbage.{sbtc_contract_name}"),
            stacks_private_key.to_string(),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidContract(_))));

        // Test config with an invalid private key
        let config = write_new_config(
            format!("{sbtc_contract_address}.{sbtc_contract_name}"),
            String::from("Garbage"),
            None,
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        );
        assert!(matches!(config, Err(Error::InvalidPrivateKey(_))));
    }

    #[test]
    fn parse_stacks_private_key_test() {
        let mut config = RawConfig::default();
        // An empty private key should fail
        assert!(matches!(
            config.parse_stacks_private_key(),
            Err(Error::InvalidPrivateKey(_))
        ));

        // An invalid key shoudl fail
        config.stacks_private_key = "This is an invalid private key...".to_string();
        assert!(matches!(
            config.parse_stacks_private_key(),
            Err(Error::InvalidPrivateKey(_))
        ));

        // A valid key should succeed
        config.stacks_private_key =
            "d655b2523bcd65e34889725c73064feb17ceb796831c0e111ba1a552b0f31b3901".to_string();
        assert_eq!(
            config.parse_stacks_private_key().unwrap().0.to_hex(),
            config.stacks_private_key
        );
    }

    #[test]
    fn parse_version_test() {
        let mut config = RawConfig::default();
        // Defaults to testnet
        let (stacks_version, bitcoin_network) = config.parse_version();
        assert_eq!(stacks_version, TransactionVersion::Mainnet);
        assert_eq!(bitcoin_network, BitcoinNetwork::Bitcoin);

        // Explicitly test Testnet
        config.network = Some(Network::Testnet);
        let (stacks_version, bitcoin_network) = config.parse_version();
        assert_eq!(stacks_version, TransactionVersion::Testnet);
        assert_eq!(bitcoin_network, BitcoinNetwork::Testnet);

        // Explicitly test Mainnet
        config.network = Some(Network::Mainnet);
        let (stacks_version, bitcoin_network) = config.parse_version();
        assert_eq!(stacks_version, TransactionVersion::Mainnet);
        assert_eq!(bitcoin_network, BitcoinNetwork::Bitcoin);
    }
    #[test]
    fn parse_contract_test() {
        let mut config = RawConfig::default();
        config.sbtc_contract = "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sbtc-alpha".to_string();
        let (parsed_contract_name, parsed_contract_address) = config.parse_contract().unwrap();
        assert_eq!(
            parsed_contract_address.to_string(),
            "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE".to_string()
        );
        assert_eq!(parsed_contract_name.to_string(), "sbtc-alpha".to_string());

        // Invalid contract
        config.sbtc_contract = "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTEsbtc-alpha".to_string();
        assert!(matches!(
            config.parse_contract(),
            Err(Error::InvalidContract(_))
        ));

        // Invalid contract address
        config.sbtc_contract = "SP3FBR2AGK5H9QBDH3EEN6DF8E.sbtc-alpha".to_string();
        assert!(matches!(
            config.parse_contract(),
            Err(Error::InvalidContract(_))
        ));

        // Invalid contract name
        config.sbtc_contract = "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.12".to_string();
        assert!(matches!(
            config.parse_contract(),
            Err(Error::InvalidContract(_))
        ));
    }
}
