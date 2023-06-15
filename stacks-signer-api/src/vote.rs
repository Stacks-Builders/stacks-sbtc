use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::transaction::Txid;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// Vote options for a transaction ballot.
pub enum VoteChoice {
    /// Approve the transaction.
    Approve,
    /// Refuse the transaction
    Refuse,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// Mechanism by which a vote was cast
pub enum VoteMechanism {
    /// The vote was cast automatically
    Auto,
    /// The vote was cast manually
    Manual,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// The status of a transaction vote
pub enum VoteStatus {
    /// The vote is incomplete and pending votes
    Pending,
    /// The vote is complete and the transaction is approved
    Approved,
    /// The vote is complete and the transaction rejected
    Rejected,
    /// The vote is complete, but consensus not reached
    NoConsensus,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// A vote header common between all vote requests
pub struct VoteRequest {
    /// The voted on transaction ID.
    txid: Txid,
    /// The public key of the signer delegator
    signing_delegator: String,
    /// The vote choice.
    vote_choice: Option<VoteChoice>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToResponse)]
/// A response for a cast vote.
pub struct VoteResponse {
    /// The caller's vote
    vote_choice: VoteChoice,
    /// The vote's current status
    vote_tally: VoteTally,
    /// A message for the caller
    message: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// The current vote tally for a transaction.
pub struct VoteTally {
    /// The percentage votes*100 required for consensus
    pub target_consensus: u64,
    /// the current consensus*100
    pub current_consensus: u64,
    /// the vote status
    pub vote_status: VoteStatus,
}
