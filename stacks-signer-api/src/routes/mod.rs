/// Key Routes
pub mod keys;
/// Signer Routes
pub mod signers;

use std::convert::Infallible;

use serde::{de::DeserializeOwned, Deserialize};
use sqlx::SqlitePool;
use warp::Filter;

#[derive(Debug, Deserialize, Clone)]
/// The query parameters for get routes that return a vector of items.
pub struct Pagination {
    /// The page number.
    pub page: Option<usize>,
    /// The limit of items per page.
    pub limit: Option<usize>,
}

/// A helper function to extract a JSON body from a request.
pub fn json_body<T: std::marker::Send + DeserializeOwned>(
) -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json::<T>())
}

/// A helper function to extract a database pool from a request.
pub fn with_pool(
    pool: SqlitePool,
) -> impl Filter<Extract = (SqlitePool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}
