use bitcoin::{consensus::Encodable, hashes::hex::ToHex, Address as BitcoinAddress};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
pub trait BitcoinNode {
    /// Broadcast the BTC transaction to the bitcoin node
    fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<(), Error>;
    /// Load the Bitcoin wallet from the given address
    fn load_wallet(&self, address: &BitcoinAddress) -> Result<(), Error>;
    /// Get all utxos from the given address
    fn list_unspent(&self, address: &BitcoinAddress) -> Result<Vec<UTXO>, Error>;
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
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Default)]
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
    bitcoind_api: String,
}

impl BitcoinNode for LocalhostBitcoinNode {
    fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<(), Error> {
        let mut writer = std::io::Cursor::new(vec![]);
        tx.consensus_encode(&mut writer)?;
        let raw_tx = writer.into_inner().to_hex();

        self.call("sendrawtransaction", vec![raw_tx])?;
        Ok(())
    }

    fn load_wallet(&self, address: &BitcoinAddress) -> Result<(), Error> {
        let result = self.create_empty_wallet();
        if let Err(Error::RPCError(message)) = &result {
            if !message.ends_with("Database already exists.\"") {
                return result;
            }
            // If the database already exists, no problem. Just emit a warning.
            warn!(message);
        }
        // Import the address
        self.import_address(address)?;
        Ok(())
    }

    /// List the UTXOs filtered on a given address.
    fn list_unspent(&self, address: &BitcoinAddress) -> Result<Vec<UTXO>, Error> {
        // Construct the params using defaults found at https://developer.bitcoin.org/reference/rpc/listunspent.html?highlight=listunspent
        let addresses = vec![address.to_string()];
        let min_conf = 0i64;
        let max_conf = 9999999i64;
        let params = (min_conf, max_conf, addresses);

        let response = self.call("listunspent", params)?;
        // Convert the response to a vector of unspent transactions
        let outputs = serde_json::from_value::<Vec<UTXO>>(response)
            .map_err(|e| Error::InvalidResponseJSON(e.to_string()))?;
        Ok(outputs)
    }
}

impl LocalhostBitcoinNode {
    pub fn new(bitcoind_api: String) -> LocalhostBitcoinNode {
        Self { bitcoind_api }
    }

    /// Make the Bitcoin RPC method call with the corresponding paramenters
    fn call(
        &self,
        method: &str,
        params: impl ureq::serde::Serialize,
    ) -> Result<serde_json::Value, Error> {
        debug!("Making Bitcoin RPC {} call...", method);
        let json_rpc =
            ureq::json!({"jsonrpc": "2.0", "id": "stx", "method": method, "params": params});
        let response = ureq::post(&self.bitcoind_api)
            .send_json(json_rpc)
            .map_err(|e| Error::RPCError(e.to_string()))?;
        let json_response = response.into_json::<serde_json::Value>()?;
        let json_result = json_response
            .get("result")
            .ok_or_else(|| Error::InvalidResponseJSON("Missing entry 'result'.".to_string()))?
            .to_owned();
        Ok(json_result)
    }

    fn create_empty_wallet(&self) -> Result<(), Error> {
        let wallet_name = "";
        let disable_private_keys = false;
        let blank = true;
        let passphrase = "";
        let avoid_reuse = false;
        let descriptors = false;
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
        debug!("Creating wallet...");
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

    fn import_address(&self, address: &BitcoinAddress) -> Result<(), Error> {
        let address = address.to_string();
        debug!("Importing address {}...", address);
        let label = "";
        let rescan = true;
        let p2sh = false;
        let params = (address, label, rescan, p2sh);
        self.call("importaddress", params)?;
        Ok(())
    }
}
