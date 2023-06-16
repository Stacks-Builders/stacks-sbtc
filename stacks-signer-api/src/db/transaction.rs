use crate::{
    db::{paginate_items, signers::add_signer, Error},
    key::Key,
    routes::keys::KeysQuery,
    signer::{Signer, Status},
};

use sqlx::{Row, SqlitePool};
use warp::http;

// SQL queries used for performing various operations on the "transactions" table.
const SQL_INSERT_KEY: &str =
    "INSERT OR REPLACE INTO transactions (signer_id, transaction) VALUES (?1, ?2)";
const SQL_DELETE_KEY: &str = "DELETE FROM transactions WHERE signer_id = ?1 AND key = ?2";
const SQL_DELETE_KEYS_BY_ID: &str = "DELETE FROM keys WHERE signer_id = ?1";
const SQL_SELECT_KEYS: &str =
    "SELECT key FROM keys WHERE signer_id = ?1 ORDER BY key ASC";
const SQL_COUNT_KEYS_BY_ID: &str =
    "SELECT COUNT(*) FROM keys WHERE signer_id = ?1";