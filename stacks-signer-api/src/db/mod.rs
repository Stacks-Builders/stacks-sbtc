#![deny(missing_docs)]

/// Module that handles keys-related database operations
pub mod keys;
/// Module that handles signers-related database operations
pub mod signers;
/// Module that handles transaction-related database operations
pub mod transaction;
/// Module that handles vote-related database operations
pub mod vote;

use sqlx::SqlitePool;

use crate::{
    db::{transaction::Error as TransactionError, vote::Error as VoteError},
    signer::Error as SignerError,
};

/// Custom error type for this database module
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Sqlite related error
    #[error("Sqlite Error: {0}")]
    SqlxError(#[from] sqlx::Error),
    /// Signer related error
    #[error("Signer Error: {0}")]
    SignerError(#[from] SignerError),
    /// Transaction related error
    #[error("Transaction Error: {0}")]
    TransactionError(#[from] TransactionError),
    /// Vote related error
    #[error("Vote Error: {0}")]
    VoteError(#[from] VoteError),
}
impl warp::reject::Reject for Error {}

// SQL schema for creating the `sbtc_signers` table
const SQL_SCHEMA_SIGNERS: &str = r#"
    CREATE TABLE IF NOT EXISTS sbtc_signers (
        signer_id INTEGER NOT NULL,
        status TEXT NOT NULL,

        PRIMARY KEY(signer_id)
    );"#;

// SQL schema for creating the `keys` table
const SQL_SCHEMA_KEYS: &str = r#"
        CREATE TABLE IF NOT EXISTS keys (
            key TEXT NOT NULL,
            signer_id INTEGER NOT NULL,
    
            PRIMARY KEY(key, signer_id),
            FOREIGN KEY(signer_id) REFERENCES sbtc_signers(signer_id) ON DELETE CASCADE
        );
        "#;
const SQL_SCHEMA_TRANSACTIONS: &str = r#"
        CREATE TABLE transactions (
            txid TEXT PRIMARY KEY,
            transaction_kind TEXT NOT NULL,
            transaction_block_height INTEGER,
            transaction_deadline_block_height INTEGER NOT NULL,
            transaction_amount INTEGER NOT NULL,
            transaction_fees INTEGER NOT NULL,
            memo BLOB NOT NULL,
            transaction_originator_address TEXT NOT NULL,
            transaction_debit_address TEXT NOT NULL,
            transaction_credit_address TEXT NOT NULL
        );
"#;

const SQL_SCHEMA_VOTE: &str = r#"
        CREATE TABLE votes (
            txid TEXT PRIMARY KEY,
            vote_status TEXT NOT NULL,
            vote_choice TEXT,
            vote_mechanism TEXT NOT NULL,
            target_consensus INTEGER NOT NULL,
            current_consensus INTEGER NOT NULL,

            FOREIGN KEY(txid) REFERENCES transactions(txid) ON DELETE CASCADE
        );
"#;

const SQL_TRANSACTION_VOTE_TRIGGER: &str = r#"CREATE TRIGGER add_empty_vote
AFTER INSERT ON transactions
FOR EACH ROW
BEGIN
    INSERT INTO votes (
        txid, vote_status, vote_choice, vote_mechanism, target_consensus, current_consensus
    ) VALUES (
        NEW.txid, 'pending', NULL, 'manual', 70, 0
    );
END;"#;

/// Initialize the database pool from the given file path or in memory if none is provided.
///
/// # Params
/// * path: Option<&str> - Optional file path to the SQLite database, or None to use in-memory storage.
///
/// # Returns
/// * Result<SqlitePool, Error>: Result containing the initialized SqlitePool, or an Error if initialization failed.
pub async fn init_pool(path: Option<String>) -> Result<SqlitePool, Error> {
    let pool = match path {
        Some(path) => SqlitePool::connect(&path).await?,
        None => SqlitePool::connect("sqlite::memory:").await?,
    };
    sqlx::query(SQL_SCHEMA_SIGNERS).execute(&pool).await?;
    sqlx::query(SQL_SCHEMA_KEYS).execute(&pool).await?;
    sqlx::query(SQL_SCHEMA_TRANSACTIONS).execute(&pool).await?;
    sqlx::query(SQL_SCHEMA_VOTE).execute(&pool).await?;
    sqlx::query(SQL_TRANSACTION_VOTE_TRIGGER)
        .execute(&pool)
        .await?;
    Ok(pool)
}

/// Paginate a slice of items.
///
/// This utility function slices a given set of items based on the specified `page` and `limit`.
/// If `page` and/or `limit` are not provided (None), the function will use default values.
///
/// # Params
/// * items: &[T] - The reference to the slice of items to be paginated.
/// * page: Option<usize> - The optional page number for pagination (1-based index).
/// * limit: Option<usize> - The optional limit representing the maximum number of items per page.
///
/// # Returns
/// * &[T]: A slice of the original items, paginated according to the provided page and limit.
pub fn paginate_items<T>(items: &[T], page: Option<usize>, limit: Option<usize>) -> &[T] {
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(usize::MAX);

    let start_index = items.len().min((page - 1) * limit);
    let end_index = items.len().min(start_index + limit);
    &items[start_index.min(end_index)..end_index]
}
