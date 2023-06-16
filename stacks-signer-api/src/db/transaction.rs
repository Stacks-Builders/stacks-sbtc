use std::str::FromStr;

use crate::{
    db::Error as DatabaseError,
    transaction::{TransactionAddress, TransactionKind},
};

use crate::transaction::Transaction;
use blockstack_lib::{burnchains::Txid, util::HexError};
use sqlx::{Row, SqlitePool};

// SQL queries used for performing various operations on the "transactions" table.
const SQL_INSERT_TRANSACTION: &str = r#"INSERT INTO transactions (
    txid, transaction_kind, transaction_block_height, transaction_deadline_block_height,
    transaction_amount, transaction_fees, memo, transaction_originator_address,
    transaction_debit_address, transaction_credit_address
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
const SQL_SELECT_TRANSACTIONS: &str = r#"SELECT * FROM transactions"#;
const SQL_SELECT_TRANSACTION_BY_ID: &str = r#"SELECT * FROM transactions WHERE txid = ?"#;

#[derive(thiserror::Error, Debug)]
/// Common errors that occur when handling Transactions.
pub enum Error {
    /// An error that can occur when parsing a Transaction id.
    #[error("Invalid Txid: {0}")]
    InvalidTxid(#[from] HexError),
    /// An error that can occur when parsing a TransactionKind.
    #[error("Invalid TransactionKind: {0}")]
    InvalidTransactionKind(String),
}

/// Add a given transaction to the database.
///
/// # Params
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
/// * transaction: Transaction - The transaction object to add to the database.
///
/// # Returns
/// * Result<(), DatabaseError>: The result of the database operation.
pub async fn add_transaction(
    pool: &SqlitePool,
    transaction: &Transaction,
) -> Result<(), DatabaseError> {
    let txid = hex::encode(transaction.txid);
    let transaction_kind = &transaction.transaction_kind.to_string();
    let transaction_block_height = transaction.transaction_block_height;
    let transaction_deadline_block_height = transaction.transaction_deadline_block_height;
    let transaction_amount = transaction.transaction_amount;
    let transaction_fees = transaction.transaction_fees;
    let memo = &transaction.memo;
    let transaction_originator_address = ""; //&transaction.transaction_originator_address.0;
    let transaction_debit_address = ""; //&transaction.transaction_debit_address.0;
    let transaction_credit_address = ""; //&transaction.transaction_credit_address.0;

    sqlx::query(SQL_INSERT_TRANSACTION)
        .bind(txid)
        .bind(transaction_kind)
        .bind(transaction_block_height.map(|height| height as i64))
        .bind(transaction_deadline_block_height as i64)
        .bind(transaction_amount as i64)
        .bind(transaction_fees as i64)
        .bind(memo)
        .bind(transaction_originator_address)
        .bind(transaction_debit_address)
        .bind(transaction_credit_address)
        .execute(pool)
        .await
        .map_err(DatabaseError::from)?;
    Ok(())
}

/// Get all transactions from the database.
///
/// # Params
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
///
/// # Returns
/// * Result<Vec<Transactions>>: The transactions found in the database.
pub async fn get_transactions(pool: &SqlitePool) -> Result<Vec<Transaction>, DatabaseError> {
    let rows = sqlx::query(SQL_SELECT_TRANSACTIONS)
        .fetch_all(pool)
        .await
        .map_err(DatabaseError::from)?;
    let mut txs = vec![];
    for row in rows {
        let txid: String = row.try_get("txid")?;
        let txid = Txid::from_hex(&txid).map_err(|e| DatabaseError::from(Error::InvalidTxid(e)))?;

        let transaction_kind: String = row.try_get("transaction_kind")?;
        let transaction_kind = TransactionKind::from_str(&transaction_kind)
            .map_err(|_| DatabaseError::from(Error::InvalidTransactionKind(transaction_kind)))?;

        let transaction_block_height: Option<i64> = row.try_get("transaction_block_height")?;
        let transaction_deadline_block_height: i64 =
            row.try_get("transaction_deadline_block_height")?;
        let transaction_amount: i64 = row.try_get("transaction_amount")?;
        let transaction_fees: i64 = row.try_get("transaction_fees")?;

        let memo: Vec<u8> = row.try_get("memo")?;
        let transaction_originator_address: String =
            row.try_get("transaction_originator_address")?;
        let transaction_originator_address =
            TransactionAddress::Bitcoin(transaction_originator_address);

        let transaction_debit_address: String = row.try_get("transaction_debit_address")?;
        let transaction_debit_address = TransactionAddress::Bitcoin(transaction_debit_address);

        let transaction_credit_address: String = row.try_get("transaction_credit_address")?;
        let transaction_credit_address = TransactionAddress::Bitcoin(transaction_credit_address);

        let tx = Transaction {
            txid,
            transaction_kind,
            transaction_block_height: transaction_block_height.map(|height| height as u64),
            transaction_deadline_block_height: transaction_deadline_block_height as u64,
            transaction_amount: transaction_amount as u64,
            transaction_fees: transaction_fees as u64,
            memo,
            transaction_originator_address,
            transaction_debit_address,
            transaction_credit_address,
        };
        txs.push(tx);
    }
    Ok(txs)
}

/// Get a transaction with a specific transaction ID from the database.
///
/// # Params
/// * txid: String - The transaction ID to search for.
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
///
/// # Returns
/// * Result<Transaction>: The transaction found in the database.
pub async fn get_transaction_by_id(
    txid: &str,
    pool: &SqlitePool,
) -> Result<Transaction, DatabaseError> {
    let row = sqlx::query(SQL_SELECT_TRANSACTION_BY_ID)
        .bind(txid)
        .fetch_one(pool)
        .await?;
    let txid: String = row.try_get("txid")?;
    let txid = Txid::from_hex(&txid).map_err(|e| DatabaseError::from(Error::from(e)))?;

    let transaction_kind: String = row.try_get("transaction_kind")?;
    let transaction_kind = TransactionKind::from_str(&transaction_kind)
        .map_err(|_| DatabaseError::from(Error::InvalidTransactionKind(transaction_kind)))?;

    let transaction_block_height: Option<i64> = row.try_get("transaction_block_height")?;
    let transaction_deadline_block_height: i64 =
        row.try_get("transaction_deadline_block_height")?;
    let transaction_amount: i64 = row.try_get("transaction_amount")?;
    let transaction_fees: i64 = row.try_get("transaction_fees")?;

    let memo: Vec<u8> = row.try_get("memo")?;
    let transaction_originator_address: String = row.try_get("transaction_originator_address")?;
    let transaction_originator_address =
        TransactionAddress::Bitcoin(transaction_originator_address);

    let transaction_debit_address: String = row.try_get("transaction_debit_address")?;
    let transaction_debit_address = TransactionAddress::Bitcoin(transaction_debit_address);

    let transaction_credit_address: String = row.try_get("transaction_credit_address")?;
    let transaction_credit_address = TransactionAddress::Bitcoin(transaction_credit_address);

    let tx = Transaction {
        txid,
        transaction_kind,
        transaction_block_height: transaction_block_height.map(|height| height as u64),
        transaction_deadline_block_height: transaction_deadline_block_height as u64,
        transaction_amount: transaction_amount as u64,
        transaction_fees: transaction_fees as u64,
        memo,
        transaction_originator_address,
        transaction_debit_address,
        transaction_credit_address,
    };
    Ok(tx)
}
