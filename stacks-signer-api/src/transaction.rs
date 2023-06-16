use rand::Rng;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::vote::{VoteChoice, VoteMechanism, VoteStatus, VoteTally};
//TODO update all these types
/// Temp bitcoin address type
pub type BitcoinAddress = String;
/// Temp stacks address type
pub type StacksAddress = String;
/// Temp txid type
pub type Txid = String;

/// Generate some dummy transactions for mocked backend
pub fn create_dummy_transactons() -> Vec<TransactionResponse> {
    let mut txs = vec![];
    let mut rng = rand::thread_rng();
    for i in 0..10 {
        let current_consensus = rng.gen_range(0..100);
        let rand_vote = rng.gen_range(0..2);
        let vote_choice = if rand_vote == 0 {
            Some(VoteChoice::Approve)
        } else if rand_vote == 1 {
            Some(VoteChoice::Reject)
        } else {
            None
        };
        let rand_kind = rng.gen_range(0..4);
        let transaction_kind = if rand_kind == 0 {
            TransactionKind::DepositReveal
        } else if rand_kind == 1 {
            TransactionKind::WithdrawalReveal
        } else if rand_kind == 2 {
            TransactionKind::WithdrawalFulfill
        } else {
            TransactionKind::WalletHandoff
        };
        let transaction_block_height = rng.gen();
        let transaction = Transaction {
            transaction_id: i.to_string(),
            transaction_kind,
            transaction_block_height,
            transaction_deadline_block_height: transaction_block_height.unwrap_or(0)
                + rng.gen_range(1..10),
            transaction_amount: rng.gen(),
            transaction_fees: rng.gen_range(10..1000),
            memo: "".to_string(),
            transaction_originator_address: TransactionAddress::Bitcoin("originator".to_string()),
            transaction_debit_address: TransactionAddress::Bitcoin(
                "escrow bitcoin wallet".to_string(),
            ),
            transaction_credit_address: TransactionAddress::Stacks(
                "sBTC protocol address".to_string(),
            ),
        };
        let tx_response = TransactionResponse {
            transaction,
            vote_tally: VoteTally {
                vote_status: VoteStatus::Pending,
                target_consensus: 70,
                current_consensus,
            },
            vote_choice,
            vote_mechanism: VoteMechanism::Manual,
        };
        txs.push(tx_response);
    }
    txs
}
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
/// The address of either a credit or debit transaction
pub enum TransactionAddress {
    /// A Bitcoin address
    Bitcoin(BitcoinAddress),
    /// A Stacks address
    Stacks(StacksAddress),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
/// The type of transaction being requested
pub enum TransactionKind {
    /// A BTC to sBTC deposit reveal transaction
    DepositReveal,
    /// An sBTC to BTC withdrawal transaction
    WithdrawalReveal,
    /// A BTC withdrawal Fullfill transaction
    WithdrawalFulfill,
    /// A sBTC wallet handoff transaction
    WalletHandoff,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            transaction_id: Default::default(),
            transaction_kind: TransactionKind::DepositReveal,
            transaction_block_height: Default::default(),
            transaction_deadline_block_height: Default::default(),
            transaction_amount: Default::default(),
            transaction_fees: Default::default(),
            memo: Default::default(),
            transaction_originator_address: TransactionAddress::Bitcoin("originator".to_string()),
            transaction_debit_address: TransactionAddress::Bitcoin(
                "escrow bitcoin wallet".to_string(),
            ),
            transaction_credit_address: TransactionAddress::Stacks(
                "sBTC protocol address".to_string(),
            ),
        }
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// A transaction
pub struct Transaction {
    /// The bitcoin transaction ID
    transaction_id: Txid,
    /// The type of transaction being voted on
    transaction_kind: TransactionKind,
    /// The height of the Bitcoin block that mined the commit transaction.
    transaction_block_height: Option<u64>,
    /// The height of the Bitcoin block at which a vote is due
    transaction_deadline_block_height: u64,
    /// The amount of sats in the transaction
    transaction_amount: u64,
    /// The amount of sats in the fee subsidy
    transaction_fees: u64,
    /// A message from the user in the transaction.
    memo: String,
    /// Originating address
    transaction_originator_address: TransactionAddress,
    /// The address of the debit wallet
    transaction_debit_address: TransactionAddress,
    /// The address of the credit account
    transaction_credit_address: TransactionAddress,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, ToResponse)]
#[response(
    description = "Transaction response returns single Transaction entity and its current vote status"
)]
/// The response returned from a transaction request
pub struct TransactionResponse {
    /// The transaction
    pub transaction: Transaction,
    /// The current vote tally of the given transaction
    pub vote_tally: VoteTally,
    /// The vote choice
    pub vote_choice: Option<VoteChoice>,
    /// The vote mechanism used
    pub vote_mechanism: VoteMechanism,
}
