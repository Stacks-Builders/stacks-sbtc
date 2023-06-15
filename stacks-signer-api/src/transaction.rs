use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToResponse, ToSchema};

use crate::vote::{VoteChoice, VoteMechanism, VoteStatus, VoteTally};
//TODO update all these types
/// Temp bitcoin address type
pub type BitcoinAddress = String;
/// Temp stacks address type
pub type StacksAddress = String;
/// Temp txid type
pub type Txid = String;
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
    HandoffTransfer,
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
    transaction: Transaction,
    vote_tally: VoteTally,
    vote_choice: Option<VoteChoice>,
    vote_mechanism: VoteMechanism,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
/// Query parameters for the transaction list
pub struct TransactionQuery {
    /// The page number.
    #[param(inline)]
    pub page: Option<usize>,
    /// The limit of signers per page.
    #[param(inline)]
    pub limit: Option<usize>,
    /// The transaction id to filter by.
    #[param(inline)]
    pub id: String,
}

/// Get transaction by id
#[utoipa::path(
        get,
        path = "/transactions/{id}",
        responses(
            (status = 200, description = "Transaction found successfully", body = TransactionResponse),
            (status = NOT_FOUND, description = "No transaction was found")
        ),
        params(
            ("id" = String, Path, description = "Transaction id for retrieving a specific Transaction"),
        )
    )]
async fn get_transaction_by_id(id: String) -> TransactionResponse {
    let mut transaction = Transaction::default();
    transaction.transaction_id = id;
    TransactionResponse {
        transaction,
        vote_tally: VoteTally {
            vote_status: VoteStatus::Pending,
            target_consensus: 7000,
            current_consensus: 131,
        },
        vote_choice: None,
        vote_mechanism: VoteMechanism::Manual,
    }
}
