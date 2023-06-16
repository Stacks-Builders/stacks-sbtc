use std::str::FromStr;

use blockstack_lib::burnchains::Txid;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
/// Vote options for a transaction ballot.
pub enum VoteChoice {
    /// Approve the transaction.
    Approve,
    /// Reject the transaction
    Reject,
}

impl FromStr for VoteChoice {
    type Err = ();

    fn from_str(input: &str) -> Result<VoteChoice, Self::Err> {
        match input.to_lowercase().as_str() {
            "approve" => Ok(VoteChoice::Approve),
            "reject" => Ok(VoteChoice::Reject),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for VoteChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VoteChoice::Approve => write!(f, "approve"),
            VoteChoice::Reject => write!(f, "reject"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
/// Mechanism by which a vote was cast
pub enum VoteMechanism {
    /// The vote was cast automatically
    Auto,
    /// The vote was cast manually
    Manual,
}

impl FromStr for VoteMechanism {
    type Err = ();

    fn from_str(input: &str) -> Result<VoteMechanism, Self::Err> {
        match input.to_lowercase().as_str() {
            "auto" => Ok(VoteMechanism::Auto),
            "manual" => Ok(VoteMechanism::Manual),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for VoteMechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VoteMechanism::Auto => write!(f, "auto"),
            VoteMechanism::Manual => write!(f, "manual"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
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

impl FromStr for VoteStatus {
    type Err = ();

    fn from_str(input: &str) -> Result<VoteStatus, Self::Err> {
        match input.to_lowercase().as_str() {
            "pending" => Ok(VoteStatus::Pending),
            "approved" => Ok(VoteStatus::Approved),
            "rejected" => Ok(VoteStatus::Rejected),
            "noconsensus" => Ok(VoteStatus::NoConsensus),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for VoteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VoteStatus::Pending => write!(f, "pending"),
            VoteStatus::Approved => write!(f, "approved"),
            VoteStatus::Rejected => write!(f, "rejected"),
            VoteStatus::NoConsensus => write!(f, "noconsensus"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// A vote request for a transaction.
pub struct VoteRequest {
    /// The hexadecimal transaction ID.
    pub txid: String,
    /// The public key of the signer delegator
    pub signing_delegator: String,
    /// The vote choice.
    pub vote_choice: VoteChoice,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToResponse)]
/// A response for a cast vote.
pub struct VoteResponse {
    /// The caller's vote
    pub vote_choice: VoteChoice,
    /// The vote's current status
    pub vote_tally: VoteTally,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// The current vote tally for a transaction.
pub struct VoteTally {
    /// The percentage votes required for consensus
    pub target_consensus: u64,
    /// the current consensus
    pub current_consensus: u64,
    /// the vote status
    pub vote_status: VoteStatus,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
/// The current vote info for a transaction
pub struct Vote {
    /// The voted on transaction ID.
    pub txid: Txid,
    /// The vote tally.
    pub vote_tally: VoteTally,
    /// The vote choice.
    pub vote_choice: Option<VoteChoice>,
    /// The current vote mechanism of the vote choice
    pub vote_mechanism: VoteMechanism,
}
