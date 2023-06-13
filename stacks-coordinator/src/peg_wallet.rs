use crate::bitcoin_node::{self, UTXO};
use crate::bitcoin_wallet::{BitcoinWallet as BitcoinWalletStruct, Error as BitcoinWalletError};
use crate::stacks_node::{self, PegOutRequestOp};
use crate::stacks_wallet::{Error as StacksWalletError, StacksWallet as StacksWalletStruct};
use bitcoin::secp256k1::PublicKey;
use bitcoin::{Address as BitcoinAddress, XOnlyPublicKey};
use blockstack_lib::types::chainstate::StacksPublicKey;
use blockstack_lib::{chainstate::stacks::StacksTransaction, types::chainstate::StacksAddress};
use std::fmt::Debug;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Stacks Wallet Error: {0}")]
    StacksWalletError(#[from] StacksWalletError),
    #[error("Bitcoin Wallet Error: {0}")]
    BitcoinWalletError(#[from] BitcoinWalletError),
}

pub trait StacksWallet {
    /// Builds a verified signed transaction for a given peg-in operation
    fn build_mint_transaction(
        &self,
        op: &stacks_node::PegInOp,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Builds a verified signed transaction for a given peg-out request operation
    fn build_burn_transaction(
        &self,
        op: &stacks_node::PegOutRequestOp,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Builds a verified signed transaction for setting the sBTC wallet public key
    fn build_set_bitcoin_wallet_public_key_transaction(
        &self,
        public_key: &PublicKey,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Builds a verified signed transaction for setting the sBTC coordinator data
    fn build_set_coordinator_data_transaction(
        &self,
        address: &StacksAddress,
        public_key: &StacksPublicKey,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Returns the sBTC address for the wallet
    fn address(&self) -> &StacksAddress;
    /// Returns the sBTC public key for the wallet
    fn public_key(&self) -> &StacksPublicKey;
}

pub trait BitcoinWallet {
    type Error: Debug;

    // Builds a fulfilled unsigned transaction using the provided utxos to cover the spend amount
    fn fulfill_peg_out(
        &self,
        op: &PegOutRequestOp,
        txouts: Vec<UTXO>,
    ) -> Result<bitcoin_node::BitcoinTransaction, Error>;

    /// Returns the BTC address for the wallet
    fn address(&self) -> &BitcoinAddress;

    fn x_only_pub_key(&self) -> &XOnlyPublicKey;
}

pub trait PegWallet {
    type StacksWallet: StacksWallet;
    type BitcoinWallet: BitcoinWallet;
    fn stacks(&self) -> &Self::StacksWallet;
    fn bitcoin(&self) -> &Self::BitcoinWallet;
}

pub struct WrapPegWallet {
    pub(crate) bitcoin_wallet: BitcoinWalletStruct,
    pub(crate) stacks_wallet: StacksWalletStruct,
}

impl PegWallet for WrapPegWallet {
    type StacksWallet = StacksWalletStruct;
    type BitcoinWallet = BitcoinWalletStruct;

    fn stacks(&self) -> &Self::StacksWallet {
        &self.stacks_wallet
    }

    fn bitcoin(&self) -> &Self::BitcoinWallet {
        &self.bitcoin_wallet
    }
}
