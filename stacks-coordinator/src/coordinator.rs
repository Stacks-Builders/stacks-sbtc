use bitcoin::{
    psbt::Prevouts,
    secp256k1::Error as Secp256k1Error,
    util::{
        base58,
        sighash::{Error as SighashError, SighashCache},
    },
    SchnorrSighashType, XOnlyPublicKey,
};
use blockstack_lib::chainstate::stacks::TransactionVersion;
use frost_coordinator::{coordinator::Error as FrostCoordinatorError, create_coordinator};
use frost_signer::net::{Error as HttpNetError, HttpNetListen};
use std::sync::{
    mpsc,
    mpsc::{RecvError, Sender},
};
use std::{thread, time};
use tracing::debug;
use wtfrost::{bip340::SchnorrProof, common::Signature};

use crate::bitcoin_wallet::BitcoinWallet;
use crate::config::{Config, Network};
use crate::peg_wallet::{
    BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError, PegWallet,
    StacksWallet as StacksWalletTrait, WrapPegWallet,
};
use crate::stacks_node::{self, Error as StacksNodeError};
use crate::stacks_wallet::StacksWallet;

// Traits in scope
use crate::bitcoin_node::{
    BitcoinNode, BitcoinTransaction, Error as BitcoinNodeError, LocalhostBitcoinNode,
};
use crate::peg_queue::{
    Error as PegQueueError, PegQueue, SbtcOp, SqlitePegQueue, SqlitePegQueueError,
};
use crate::stacks_node::{client::NodeClient, StacksNode};

type FrostCoordinator = frost_coordinator::coordinator::Coordinator<HttpNetListen>;

pub type PublicKey = XOnlyPublicKey;

/// Helper that uses this module's error type
pub type Result<T> = std::result::Result<T, Error>;

/// Kinds of common errors used by stacks coordinator
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error occurred with the HTTP Relay
    #[error("Http Network Error: {0}")]
    HttpNetError(#[from] HttpNetError),
    /// Error occurred in the Peg Queue
    #[error("Peg Queue Error: {0}")]
    PegQueueError(#[from] PegQueueError),
    // Error occurred in the Peg Wallet
    #[error("Peg Wallet Error: {0}")]
    PegWalletError(#[from] PegWalletError),
    /// Error occurred in the Frost Coordinator
    #[error("Frost Coordinator Error: {0}")]
    FrostCoordinatorError(#[from] FrostCoordinatorError),
    /// Error occurred in the Sqlite Peg Queue
    #[error("Sqlite Peg Queue Error: {0}")]
    SqlitePegQueueError(#[from] SqlitePegQueueError),
    #[error("Command sender disconnected unexpectedly: {0}")]
    UnexpectedSenderDisconnect(#[from] RecvError),
    #[error("Stacks Node Error: {0}")]
    StacksNodeError(#[from] StacksNodeError),
    #[error("Bitcoin Node Error: {0}")]
    BitcoinNodeError(#[from] BitcoinNodeError),
    #[error("{0}")]
    ConfigError(String),
    #[error(
        "Invalid generated aggregate public key. Frost coordinator/signers may be misconfigured."
    )]
    InvalidPublicKey(#[from] Secp256k1Error),
    #[error("Error occured during signing: {0}")]
    SigningError(#[from] SighashError),
}

pub trait Coordinator: Sized {
    type PegQueue: PegQueue;
    type FeeWallet: PegWallet;
    type StacksNode: StacksNode;
    type BitcoinNode: BitcoinNode;

    // Required methods
    fn peg_queue(&self) -> &Self::PegQueue;
    fn fee_wallet_mut(&mut self) -> &mut Self::FeeWallet;
    fn fee_wallet(&self) -> &Self::FeeWallet;
    fn frost_coordinator(&self) -> &FrostCoordinator;
    fn frost_coordinator_mut(&mut self) -> &mut FrostCoordinator;
    fn stacks_node(&self) -> &Self::StacksNode;
    fn bitcoin_node(&self) -> &Self::BitcoinNode;

    // Provided methods
    fn run(mut self) -> Result<()> {
        let (sender, receiver) = mpsc::channel::<Command>();
        Self::poll_ping_thread(sender);

        loop {
            match receiver.recv()? {
                Command::Stop => break,
                Command::Timeout => {
                    self.peg_queue().poll(self.stacks_node())?;
                    self.process_queue()?;
                }
            }
        }
        Ok(())
    }

    fn poll_ping_thread(sender: Sender<Command>) {
        thread::spawn(move || loop {
            sender
                .send(Command::Timeout)
                .expect("thread send error {0}");
            thread::sleep(time::Duration::from_millis(500));
        });
    }

    fn process_queue(&mut self) -> Result<()> {
        match self.peg_queue().sbtc_op()? {
            Some(SbtcOp::PegIn(op)) => self.peg_in(op),
            Some(SbtcOp::PegOutRequest(op)) => self.peg_out(op),
            None => Ok(()),
        }
    }
}

// Private helper functions
trait CoordinatorHelpers: Coordinator {
    fn peg_in(&mut self, op: stacks_node::PegInOp) -> Result<()> {
        // Retrieve the nonce from the stacks node using the sBTC wallet address
        let nonce = self
            .stacks_node()
            .next_nonce(self.fee_wallet().stacks().address())?;

        // Build a mint transaction using the peg in op and calculated nonce
        let tx = self
            .fee_wallet()
            .stacks()
            .build_mint_transaction(&op, nonce)?;

        // Broadcast the resulting sBTC transaction to the stacks node
        self.stacks_node().broadcast_transaction(&tx)?;
        Ok(())
    }

    fn peg_out(&mut self, op: stacks_node::PegOutRequestOp) -> Result<()> {
        // Retrieve the nonce from the stacks node using the sBTC wallet address
        let nonce = self
            .stacks_node()
            .next_nonce(self.fee_wallet().stacks().address())?;

        // Build a burn transaction using the peg out request op and calculated nonce
        let burn_tx = self
            .fee_wallet()
            .stacks()
            .build_burn_transaction(&op, nonce)?;

        // Broadcast the resulting sBTC transaction to the stacks node
        self.stacks_node().broadcast_transaction(&burn_tx)?;

        // Build and sign a fulfilled bitcoin transaction
        let fulfill_tx = self.fulfill_peg_out(&op)?;

        // Broadcast the resulting BTC transaction to the Bitcoin node
        self.bitcoin_node().broadcast_transaction(&fulfill_tx)?;
        Ok(())
    }

    fn fulfill_peg_out(&mut self, op: &stacks_node::PegOutRequestOp) -> Result<BitcoinTransaction> {
        // Retreive the utxos
        let utxos = self
            .bitcoin_node()
            .list_unspent(self.fee_wallet().bitcoin().address())?;

        // Build unsigned fulfilled peg out transaction
        let mut tx = self.fee_wallet().bitcoin().fulfill_peg_out(op, utxos)?;

        // Sign the transaction
        for index in 0..tx.input.len() {
            let mut comp = SighashCache::new(&tx);

            let taproot_sighash = comp
                .taproot_signature_hash(
                    index,
                    &Prevouts::All(&tx.output),
                    None,
                    None,
                    SchnorrSighashType::All,
                )
                .map_err(Error::SigningError)?;
            let (_frost_sig, schnorr_proof) = self
                .frost_coordinator_mut()
                .sign_message(&taproot_sighash)?;

            debug!(
                "Fulfill Tx {:?} SchnorrProof ({},{})",
                &tx, schnorr_proof.r, schnorr_proof.s
            );

            let finalized = [
                schnorr_proof.to_bytes().as_ref(),
                &[SchnorrSighashType::All as u8],
            ]
            .concat();
            let finalized_b58 = base58::encode_slice(&finalized);
            debug!("CALC SIG ({}) {}", finalized.len(), finalized_b58);

            tx.input[index].witness.push(finalized);
        }
        //Return the signed transaction
        Ok(tx)
    }
}

impl<T: Coordinator> CoordinatorHelpers for T {}

pub enum Command {
    Stop,
    Timeout,
}

pub struct StacksCoordinator {
    frost_coordinator: FrostCoordinator,
    local_peg_queue: SqlitePegQueue,
    local_stacks_node: NodeClient,
    local_bitcoin_node: LocalhostBitcoinNode,
    pub local_fee_wallet: WrapPegWallet,
}

impl StacksCoordinator {
    pub fn run_dkg_round(&mut self) -> Result<PublicKey> {
        let p = self.frost_coordinator.run_distributed_key_generation()?;
        PublicKey::from_slice(&p.x().to_bytes()).map_err(Error::InvalidPublicKey)
    }

    pub fn sign_message(&mut self, message: &str) -> Result<(Signature, SchnorrProof)> {
        Ok(self.frost_coordinator.sign_message(message.as_bytes())?)
    }
}

impl TryFrom<Config> for StacksCoordinator {
    type Error = Error;
    fn try_from(mut config: Config) -> Result<Self> {
        // Determine what network we are running on
        let (version, bitcoin_network) = match config.network.as_ref().unwrap_or(&Network::Mainnet)
        {
            Network::Mainnet => (TransactionVersion::Mainnet, bitcoin::Network::Bitcoin),
            Network::Testnet => (TransactionVersion::Testnet, bitcoin::Network::Testnet),
        };

        // Create the frost coordinator and use it to generate the aggregate public key and corresponding bitcoin wallet address
        // Note: all errors returned from create_coordinator relate to configuration issues. Convert to this error.
        let mut frost_coordinator =
            create_coordinator(&config.signer_config_path).map_err(|e| {
                Error::ConfigError(format!(
                    "Invalid signer_config_path {:?}: {}",
                    &config.signer_config_path, e
                ))
            })?;
        frost_coordinator.run_distributed_key_generation()?;
        // This should not be run on startup unless required:
        // 1. No aggregate public key stored in persitent storage anywhere
        // 2. no address already set in sbtc contract (get-bitcoin-wallet-address)
        let pubkey = frost_coordinator.get_aggregate_public_key()?;
        let xonly_pubkey =
            PublicKey::from_slice(&pubkey.x().to_bytes()).map_err(Error::InvalidPublicKey)?;

        let local_stacks_node = NodeClient::new(&config.stacks_node_rpc_url);
        // If a user has not specified a start block height, begin from the current burn block height by default
        let burn_block_height = local_stacks_node.burn_block_height()?;
        config.start_block_height = config.start_block_height.or(Some(burn_block_height));

        // Create the bitcoin and stacks wallets
        let bitcoin_wallet = BitcoinWallet::new(xonly_pubkey, bitcoin_network);
        let stacks_wallet = StacksWallet::new(
            config.sbtc_contract.clone(),
            &config.stacks_private_key,
            version,
            10,
        )
        .map_err(|e| Error::ConfigError(e.to_string()))?;

        // Set the bitcoin address using the sbtc contract
        let nonce = local_stacks_node.next_nonce(stacks_wallet.address())?;
        let tx =
            stacks_wallet.build_set_btc_address_transaction(bitcoin_wallet.address(), nonce)?;
        local_stacks_node.broadcast_transaction(&tx)?;

        let local_bitcoin_node = LocalhostBitcoinNode::new(config.bitcoin_node_rpc_url.clone());
        local_bitcoin_node.load_wallet(bitcoin_wallet.address())?;

        let local_fee_wallet = WrapPegWallet {
            bitcoin_wallet,
            stacks_wallet,
        };
        let local_peg_queue = SqlitePegQueue::try_from(&config)?;

        Ok(Self {
            local_peg_queue,
            local_stacks_node,
            local_bitcoin_node,
            frost_coordinator,
            local_fee_wallet,
        })
    }
}

impl Coordinator for StacksCoordinator {
    type PegQueue = SqlitePegQueue;
    type FeeWallet = WrapPegWallet;
    type StacksNode = NodeClient;
    type BitcoinNode = LocalhostBitcoinNode;

    fn peg_queue(&self) -> &Self::PegQueue {
        &self.local_peg_queue
    }

    fn fee_wallet_mut(&mut self) -> &mut Self::FeeWallet {
        &mut self.local_fee_wallet
    }

    fn fee_wallet(&self) -> &Self::FeeWallet {
        &self.local_fee_wallet
    }

    fn frost_coordinator(&self) -> &FrostCoordinator {
        &self.frost_coordinator
    }

    fn frost_coordinator_mut(&mut self) -> &mut FrostCoordinator {
        &mut self.frost_coordinator
    }

    fn stacks_node(&self) -> &Self::StacksNode {
        &self.local_stacks_node
    }

    fn bitcoin_node(&self) -> &Self::BitcoinNode {
        &self.local_bitcoin_node
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::coordinator::{CoordinatorHelpers, StacksCoordinator};
    use crate::stacks_node::PegOutRequestOp;
    use bitcoin::consensus::Encodable;
    use blockstack_lib::burnchains::Txid;
    use blockstack_lib::chainstate::stacks::address::{PoxAddress, PoxAddressType20};
    use blockstack_lib::types::chainstate::BurnchainHeaderHash;

    #[ignore]
    #[test]
    fn btc_fulfill_peg_out() {
        let config = Config {
            sbtc_contract: "".to_string(),
            stacks_private_key: "".to_string(),
            stacks_node_rpc_url: "".to_string(),
            bitcoin_node_rpc_url: "".to_string(),
            frost_dkg_round_id: 0,
            signer_config_path: "conf/signer.toml".to_string(),
            start_block_height: None,
            rusqlite_path: None,
            network: None,
            transaction_fee: 10,
        };
        // todo: make StacksCoordinator with mock FrostCoordinator to locally generate PublicKey and Signature for unit test
        let mut sc = StacksCoordinator::try_from(config).unwrap();
        let recipient = PoxAddress::Addr20(false, PoxAddressType20::P2WPKH, [0; 20]);
        let peg_wallet_address = PoxAddress::Addr20(false, PoxAddressType20::P2WPKH, [0; 20]);
        let op = PegOutRequestOp {
            amount: 0,
            recipient: recipient,
            signature: blockstack_lib::util::secp256k1::MessageSignature([0; 65]),
            peg_wallet_address: peg_wallet_address,
            fulfillment_fee: 0,
            memo: vec![],
            txid: Txid([0; 32]),
            vtxindex: 0,
            block_height: 0,
            burn_header_hash: BurnchainHeaderHash([0; 32]),
        };
        let btc_tx_result = sc.fulfill_peg_out(&op);
        assert!(btc_tx_result.is_ok());
        let btc_tx = btc_tx_result.unwrap();
        let mut btc_tx_encoded: Vec<u8> = vec![];
        btc_tx.consensus_encode(&mut btc_tx_encoded).unwrap();
        let verify_result = bitcoin::bitcoinconsensus::verify(&[], 100, &btc_tx_encoded, 0);
        assert!(verify_result.is_ok())
    }
}
