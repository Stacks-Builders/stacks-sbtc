use serde::{Deserialize, Serialize};

use std::str::FromStr;

#[derive(thiserror::Error, Debug)]
/// Common errors that occur when handling Signers.
pub enum Error {
    /// An error that can occur when parsing a Status.
    #[error("Invalid Status Error: {0}")]
    InvalidStatusError(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
/// The current status of a signer.
pub enum Status {
    /// The signer is active and can be used to sign transactions.
    // This implies that the signer has successfully registered its key and is ready to sign.
    Active,
    /// The signer is inactive and not available to sign transactions.
    // This implies that the singer has either not successfully reigstered its key or has
    // removed itself from the registered signers and is not participating in signing rounds.
    Inactive,
}

impl Status {
    /// Returns the string representation of the Status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

impl FromStr for Status {
    type Err = Error;
    /// Parses a string into a Status.
    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "inactive" => Self::Inactive,
            other => return Err(Error::InvalidStatusError(other.to_owned())),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
/// A signer is a user that can participate in a signing round to sign transactions.
pub struct Signer {
    /// The signer's ID.
    pub signer_id: i64,
    /// The user's ID.
    pub user_id: i64,
    /// The current status of the signer.
    pub status: Status,
}
