use crate::bitcoin_node::UTXO;
use crate::coordinator::PublicKey;
use crate::peg_wallet::{BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError};
use crate::stacks_node::PegOutRequestOp;
use bitcoin::XOnlyPublicKey;
use bitcoin::{
    hashes::hex::FromHex, secp256k1::Secp256k1, Address, Network, OutPoint, Script, Transaction,
    TxIn,
};
use tracing::{debug, warn};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Unable to fulfill peg-out request op due to insufficient funds.")]
    InsufficientFunds,
    #[error("Invalid unspent transaction id: {0}")]
    InvalidTransactionID(String),
    #[error("Missing peg-out fulfillment utxo.")]
    MissingFulfillmentUTXO,
    #[error("Fulfillment UTXO amount does not equal the fulfillment fee.")]
    MismatchedFulfillmentFee,
}

pub struct BitcoinWallet {
    address: Address,
    public_key: PublicKey,
}

impl BitcoinWallet {
    pub fn new(public_key: PublicKey, network: Network) -> Self {
        let secp = Secp256k1::verification_only();
        let address = bitcoin::Address::p2tr(&secp, public_key, None, network);
        Self {
            address,
            public_key,
        }
    }
}

/// Minimum dust required
const DUST_UTXO_LIMIT: u64 = 5500;

impl BitcoinWalletTrait for BitcoinWallet {
    type Error = Error;
    fn fulfill_peg_out(
        &self,
        op: &PegOutRequestOp,
        available_utxos: Vec<UTXO>,
    ) -> Result<Transaction, PegWalletError> {
        // Create an empty transaction
        let mut tx = Transaction {
            version: 2,
            lock_time: bitcoin::PackedLockTime(0),
            input: vec![],
            output: vec![],
        };
        // Consume UTXOs until we have enough to cover the total spend (fulfillment fee and peg out amount)
        let mut total_consumed = 0;
        let mut utxos = vec![];
        let mut fulfillment_utxo = None;
        for utxo in available_utxos.into_iter() {
            if utxo.txid == op.txid.to_string() && utxo.vout == 2 {
                // This is the fulfillment utxo.
                if utxo.amount != op.fulfillment_fee {
                    // Something is wrong. The fulfillment fee should match the fulfillment utxo amount.
                    // Malformed Peg Request Op
                    return Err(PegWalletError::from(Error::MismatchedFulfillmentFee));
                }
                fulfillment_utxo = Some(utxo);
            } else if total_consumed < op.amount {
                total_consumed += utxo.amount;
                utxos.push(utxo);
            } else if fulfillment_utxo.is_some() {
                // We have consumed enough to cover the total spend
                // i.e. have found the fulfillment utxo and covered the peg out amount
                break;
            }
        }
        // Sanity check all the things!
        // If we did not find the fulfillment utxo, something went wrong
        let fulfillment_utxo = fulfillment_utxo.ok_or_else(|| {
            warn!("Failed to find fulfillment utxo.");
            Error::MissingFulfillmentUTXO
        })?;
        // Check that we have sufficient funds and didn't just run out of available utxos.
        if total_consumed < op.amount {
            warn!(
                "Consumed total {} is less than intended spend: {}",
                total_consumed, op.amount
            );
            return Err(PegWalletError::from(Error::InsufficientFunds));
        }
        // Get the transaction change amount
        let change_amount = total_consumed - op.amount;
        debug!(
            "change_amount: {:?}, total_consumed: {:?}, op.amount: {:?}",
            change_amount, total_consumed, op.amount
        );
        if change_amount >= DUST_UTXO_LIMIT {
            let secp = Secp256k1::verification_only();
            let script_pubkey = Script::new_v1_p2tr(&secp, self.public_key, None);
            let change_output = bitcoin::TxOut {
                value: change_amount,
                script_pubkey,
            };
            tx.output.push(change_output);
        } else {
            // Instead of leaving that change to the BTC miner, we could / should bump the sortition fee
            debug!("Not enough change to clear dust limit. Not adding change address.");
        }
        // Convert the utxos to inputs for the transaction, ensuring the fulfillment utxo is the first input
        let fulfillment_input = utxo_to_input(fulfillment_utxo)?;
        tx.input.push(fulfillment_input);
        for utxo in utxos {
            let input = utxo_to_input(utxo)?;
            tx.input.push(input);
        }
        Ok(tx)
    }

    fn address(&self) -> &Address {
        &self.address
    }

    fn x_only_pub_key(&self) -> &XOnlyPublicKey {
        &self.public_key
    }
}

// Helper function to convert a utxo to an unsigned input
fn utxo_to_input(utxo: UTXO) -> Result<TxIn, Error> {
    let input = TxIn {
        previous_output: OutPoint {
            txid: bitcoin::Txid::from_hex(&utxo.txid)
                .map_err(|_| Error::InvalidTransactionID(utxo.txid))?,
            vout: utxo.vout,
        },
        script_sig: Default::default(),
        sequence: bitcoin::Sequence(0xFFFFFFFD), // allow RBF
        witness: Default::default(),
    };
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::{BitcoinWallet, Error};
    use crate::bitcoin_node::UTXO;
    use crate::coordinator::PublicKey;
    use crate::peg_wallet::{BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError};
    use crate::util::test::{build_peg_out_request_op, PRIVATE_KEY_HEX};
    use hex::encode;
    use rand::Rng;
    use std::str::FromStr;

    /// Helper function to build a valid bitcoin wallet
    fn bitcoin_wallet() -> BitcoinWallet {
        let public_key =
            PublicKey::from_str("cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115")
                .expect("Failed to construct a valid public key for the bitcoin wallet");
        BitcoinWallet::new(public_key, bitcoin::Network::Testnet)
    }

    /// Helper function for building a random txid (32 byte hex string)
    fn generate_txid() -> String {
        let data: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        encode(data)
    }

    /// Helper function for building a utxo with the given txid, vout, and amount
    fn build_utxo(txid: String, vout: u32, amount: u64) -> UTXO {
        UTXO {
            txid,
            vout,
            amount,
            ..Default::default()
        }
    }

    /// Helper function for building a vector of nmb utxos with amounts increasing by 10000
    fn build_utxos(nmb: u32) -> Vec<UTXO> {
        (1..=nmb)
            .map(|i| build_utxo(generate_txid(), i, i as u64 * 10000))
            .collect()
    }

    #[test]
    fn fulfill_peg_out_insufficient_funds() {
        let wallet = bitcoin_wallet();
        let amount = 200000;

        // (1+2+3+4+5)*10000 = 1500000 < 200000. Insufficient funds.
        let mut txouts = build_utxos(5);

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op
        let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo);

        let result = wallet.fulfill_peg_out(&op, txouts);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            PegWalletError::BitcoinWalletError(Error::InsufficientFunds)
        );
    }

    #[test]
    fn fulfill_peg_out_change() {
        let wallet = bitcoin_wallet();
        let amount = 200000;

        // (1+2+3+4+5)*10000 = 210000 > 200000. We have change of 10000
        let mut txouts = build_utxos(6); // (1+2+3+4+5+6)*10000 = 210000

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op
        let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo);

        let btc_tx = wallet.fulfill_peg_out(&op, txouts).unwrap();
        assert_eq!(btc_tx.input.len(), 7);
        assert_eq!(btc_tx.output.len(), 1); // We have change!
        assert_eq!(btc_tx.output[0].value, 10000);
    }

    #[test]
    fn fulfill_peg_out_no_change() {
        let wallet = bitcoin_wallet();
        let amount = 9999;

        // 1*10000 = 10000 > 9999. We only have change of 1...not enough to cover dust
        let mut txouts = build_utxos(1); // 1*10000 = 10000

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op
        let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo);

        let btc_tx = wallet.fulfill_peg_out(&op, txouts).unwrap();
        assert_eq!(btc_tx.input.len(), 2);
        assert_eq!(btc_tx.output.len(), 0); // No change!
    }

    #[test]
    fn fulfill_peg_out_missing_fulfillment_utxo() {
        let wallet = bitcoin_wallet();
        let amount = 9999;

        let mut txouts = vec![];

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op, but with an invalid vout (i.e. incorrect vout)
        let fulfillment_utxo_invalid_vout = build_utxo(op.txid.to_string(), 1, 1);
        // Build a fulfillment utxo that does not match the generated op (i.e. mismatched txid)
        let fulfillment_utxo_invalid_txid = build_utxo(generate_txid(), 2, 1);
        txouts.push(fulfillment_utxo_invalid_vout);
        txouts.push(fulfillment_utxo_invalid_txid);

        let result = wallet.fulfill_peg_out(&op, txouts);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            PegWalletError::BitcoinWalletError(Error::MissingFulfillmentUTXO)
        );
    }

    #[test]
    fn fulfill_peg_out_mismatched_fulfillment_utxo() {
        let wallet = bitcoin_wallet();
        let amount = 9999;

        let mut txouts = vec![];

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 10);
        // Build a fulfillment utxo that matches the generated op, but has an invalid amount (does not cover the fulfillment fee)
        let fulfillment_utxo_invalid_amount = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo_invalid_amount);

        let result = wallet.fulfill_peg_out(&op, txouts);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            PegWalletError::BitcoinWalletError(Error::MismatchedFulfillmentFee)
        );
    }
}
