use std::{borrow::Cow, str::FromStr};

use bdk::descriptor::calc_checksum;
use bitcoin::{consensus::Encodable, hashes::sha256d::Hash, Txid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, warn};
use url::Url;

use crate::peg_wallet::BitcoinWallet;
pub trait BitcoinNode {
    /// Broadcast the BTC transaction to the bitcoin node
    fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<Txid, Error>;
    /// Load the Bitcoin wallet from the given address
    fn load_wallet(&self, address: &impl BitcoinWallet) -> Result<(), Error>;
    /// Get all utxos from the given address
    fn list_unspent(&self, address: &impl BitcoinWallet) -> Result<Vec<UTXO>, Error>;
}

pub type BitcoinTransaction = bitcoin::Transaction;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("RPC Error: {0}")]
    RPCError(String),
    #[error("{0}")]
    InvalidResponseJSON(String),
    #[error("Invalid utxo: {0}")]
    InvalidUTXO(String),
    #[error("Invalid transaction hash")]
    InvalidTxHash,
    #[error("Could not compute descriptor checksum: {0}")]
    DescriptorError(#[from] bdk::descriptor::error::Error),
    #[error("URL Parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub address: String,
    pub label: String,
    pub scriptPubKey: String,
    pub amount: u64,
    pub confirmations: u64,
    pub redeemScript: String,
    pub witnessScript: String,
    pub spendable: bool,
    pub solvable: bool,
    pub reused: bool,
    pub desc: String,
    pub safe: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Wallet {
    name: String,
    warning: String,
}

pub struct LocalhostBitcoinNode {
    bitcoind_api: Url,
    wallet_name: String,
}

impl BitcoinNode for LocalhostBitcoinNode {
    fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<Txid, Error> {
        let mut tx_bytes: Vec<u8> = vec![];
        tx.consensus_encode(&mut tx_bytes)?;
        let raw_tx = hex::encode(&tx_bytes);

        let result = self
            .call("sendrawtransaction", [&raw_tx])?
            .as_str()
            .ok_or(Error::InvalidResponseJSON(
                "No transaction hash in sendrawtransaction response".to_string(),
            ))?
            .to_string();

        Ok(Txid::from_hash(
            Hash::from_str(&result).map_err(|_| Error::InvalidTxHash)?,
        ))
    }

    fn load_wallet(&self, address: &impl BitcoinWallet) -> Result<(), Error> {
        debug!("Loading bitcoin wallet...");
        let result = self.create_empty_wallet();
        if let Err(Error::RPCError(message)) = &result {
            if message.contains("Database already exists") {
                // If the database already exists, no problem.
                info!("Wallet already exists");
            } else {
                return result;
            }
        }
        // Import the address
        self.import_address(address)?;
        Ok(())
    }

    /// List the UTXOs filtered on a given address.
    fn list_unspent(&self, wallet: &impl BitcoinWallet) -> Result<Vec<UTXO>, Error> {
        debug!("Retrieving utxos...");
        // Construct the params using defaults found at https://developer.bitcoin.org/reference/rpc/listunspent.html?highlight=listunspent
        let addresses: Vec<String> = vec![wallet.address().to_string()];
        let min_conf = 0i64;
        let max_conf = 9999999i64;
        let params = (min_conf, max_conf, addresses);

        let response = self.call_wallet("listunspent", params)?;

        // Convert the response to a vector of unspent transactions
        let result: Result<Vec<UTXO>, Error> = response
            .as_array()
            .ok_or(Error::InvalidResponseJSON(
                "Listunspent response is not an array".to_string(),
            ))?
            .iter()
            .map(Self::raw_to_utxo)
            .collect();

        result
    }
}

impl LocalhostBitcoinNode {
    pub fn new(bitcoind_api: Url) -> LocalhostBitcoinNode {
        Self {
            bitcoind_api,
            wallet_name: "stacks_coordinator".to_string(),
        }
    }

    /// Make the Bitcoin RPC method call with the corresponding paramenters
    fn call(
        &self,
        method: &str,
        params: impl ureq::serde::Serialize,
    ) -> Result<serde_json::Value, Error> {
        self.call_path(method, params, None)
    }

    /// Make the Bitcoin RPC method call against the "/wallet/<self.wallet_name" with the corresponding paramenters
    fn call_wallet(
        &self,
        method: &str,
        params: impl ureq::serde::Serialize,
    ) -> Result<serde_json::Value, Error> {
        self.call_path(
            method,
            params,
            Some(&format!("/wallet/{}", self.wallet_name)),
        )
    }

    /// Make the Bitcoin RPC method call against the specified path with the corresponding paramenters
    fn call_path(
        &self,
        method: &str,
        params: impl ureq::serde::Serialize,
        path: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        debug!("Making Bitcoin RPC {} call...", method);
        let json_rpc =
            ureq::json!({"jsonrpc": "2.0", "id": "stx", "method": method, "params": params});

        let url = if let Some(path) = path {
            Cow::Owned(self.bitcoind_api.join(path)?)
        } else {
            Cow::Borrowed(&self.bitcoind_api)
        };

        let response = ureq::post(&url.to_string())
            .send_json(json_rpc)
            .map_err(|e| Error::RPCError(parse_rpc_error(e)))?;

        let json_response = response.into_json::<serde_json::Value>()?;
        let json_result = json_response
            .get("result")
            .ok_or_else(|| Error::InvalidResponseJSON("Missing entry 'result'.".to_string()))?
            .to_owned();
        Ok(json_result)
    }

    fn create_empty_wallet(&self) -> Result<(), Error> {
        debug!("Creating wallet...");
        let wallet_name = &self.wallet_name;
        let disable_private_keys = true;
        let blank = true;
        let passphrase = "";
        let avoid_reuse = false;
        let descriptors = true;
        let load_on_startup = true;
        let params = (
            wallet_name,
            disable_private_keys,
            blank,
            passphrase,
            avoid_reuse,
            descriptors,
            load_on_startup,
        );
        let wallet = serde_json::from_value::<Wallet>(self.call("createwallet", params)?)
            .map_err(|e| Error::InvalidResponseJSON(e.to_string()))?;
        if !wallet.warning.is_empty() {
            warn!(
                "Wallet {} was not loaded cleanly: {}",
                wallet.name, wallet.warning
            );
        }
        Ok(())
    }

    fn import_address(&self, wallet: &impl BitcoinWallet) -> Result<(), Error> {
        debug!("Importing address {}...", wallet.address());

        // Create a descriptor using a Bech32 (segwit) Pay-to-Taproot (P2TR) address.
        let desc = {
            let pub_hex = hex::encode(wallet.x_only_pub_key().serialize());
            let descriptor = format!("tr({})", pub_hex);
            let checksum = calc_checksum(&descriptor)?;

            format!("{}#{}", descriptor, checksum)
        };

        let timestamp = "now";
        // let range = 1; // Set to 1 to ensure only one address is imported
        let internal = false;
        let watchonly = true;
        let label = "";
        let keypool = true;
        let rescan = true;
        let descriptor_object = serde_json::json!({
            "desc": desc,
            // "range": range,
            "timestamp": timestamp,
            "internal": internal,
            "watchonly": watchonly,
            "label": label,
            "keypool": keypool,
            "rescan": rescan
        });
        let params = (serde_json::json!([descriptor_object]),);
        self.call_wallet("importdescriptors", params)?;
        Ok(())
    }

    fn raw_to_utxo(raw: &Value) -> Result<UTXO, Error> {
        Ok(UTXO {
            txid: raw["txid"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse txid".to_string(),
                ))?
                .to_string(),
            vout: raw["vout"].as_u64().ok_or(Error::InvalidResponseJSON(
                "Could not parse vout".to_string(),
            ))? as u32,
            address: raw["address"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse address".to_string(),
                ))?
                .to_string(),
            label: raw["label"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse label".to_string(),
                ))?
                .to_string(),
            scriptPubKey: raw["scriptPubKey"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse scriptPubKey".to_string(),
                ))?
                .to_string(),
            amount: raw["amount"].as_f64().map(|amount| amount as u64).ok_or(
                Error::InvalidResponseJSON("Could not parse amount".to_string()),
            )?,
            confirmations: raw["confirmations"]
                .as_u64()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse confirmations".to_string(),
                ))?,
            redeemScript: "".to_string(),
            witnessScript: "".to_string(),
            spendable: raw["spendable"]
                .as_bool()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse spendable".to_string(),
                ))?,
            solvable: raw["solvable"].as_bool().ok_or(Error::InvalidResponseJSON(
                "Could not parse solvable".to_string(),
            ))?,
            reused: false,
            desc: "".to_string(),
            safe: raw["safe"].as_bool().ok_or(Error::InvalidResponseJSON(
                "Could not parse safe".to_string(),
            ))?,
        })
    }
}

fn parse_rpc_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(status, response) => format!(
            "{} {}",
            status,
            response.into_string().unwrap_or_else(|e| e.to_string())
        ),
        ureq::Error::Transport(err) => err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn should_map_json_to_utxo() {
        let value = json!({
            "address": "bcrt1qykqup0h6ry9x3c89llzpznrvm9nfd7fqwnt0hu",
            "amount": 50.00000000,
            "confirmations": 123,
            "label": "",
            "parent_descs": [],
            "safe": true,
            "scriptPubKey": "00142581c0befa190a68e0e5ffc4114c6cd96696f920",
            "solvable": false,
            "spendable": false,
            "txid": "19b7fb5fd6dc25b76aeedb812b7fdc7bf8fac343913706c8b39d23ef7375860c",
            "vout": 0,
        });

        let res = LocalhostBitcoinNode::raw_to_utxo(&value).unwrap();

        assert_eq!(
            res,
            UTXO {
                txid: "19b7fb5fd6dc25b76aeedb812b7fdc7bf8fac343913706c8b39d23ef7375860c"
                    .to_string(),
                vout: 0,
                address: "bcrt1qykqup0h6ry9x3c89llzpznrvm9nfd7fqwnt0hu".to_string(),
                label: "".to_string(),
                scriptPubKey: "00142581c0befa190a68e0e5ffc4114c6cd96696f920".to_string(),
                amount: 50,
                confirmations: 123,
                redeemScript: "".to_string(),
                witnessScript: "".to_string(),
                spendable: false,
                solvable: false,
                reused: false,
                desc: "".to_string(),
                safe: true,
            }
        );
    }
}
