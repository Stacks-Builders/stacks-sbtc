#![deny(missing_docs)]

/// Key Routes
pub mod keys;
/// Signer Routes
pub mod signers;
/// Transaction Routes
pub mod transaction;
/// Vote Routes
pub mod vote;

use std::convert::Infallible;

use serde::{de::DeserializeOwned, Deserialize};
use sqlx::SqlitePool;
use warp::{Filter, Rejection, Reply};

use self::{
    keys::{add_key_route, delete_key_route, get_keys_route},
    signers::{add_signer_route, delete_signer_route, get_signers_route},
    transaction::{get_transaction_by_id_route, get_transactions_route},
    vote::vote_route,
};

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

/// A helper function to combine all routes into one Warp Filter
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone: The Warp filter for the routes.
pub fn all_routes(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    // Set up the routes
    // Key routes
    let add_key_route = add_key_route(pool.clone());
    let delete_key_route = delete_key_route(pool.clone());
    let get_key_route = get_keys_route(pool.clone());
    // Signer routes
    let add_signer_route = add_signer_route(pool.clone());
    let delete_signer_route = delete_signer_route(pool.clone());
    let get_signers_route = get_signers_route(pool.clone());
    // Transaction routes
    let get_transactions_route = get_transactions_route(pool.clone());
    let get_transaction_by_id_route = get_transaction_by_id_route(pool.clone());
    // Vote routes
    let vote_route = vote_route(pool);

    // Combine and return the routes in a single filter
    add_key_route
        .or(delete_key_route)
        .or(get_key_route)
        .or(add_signer_route)
        .or(delete_signer_route)
        .or(get_signers_route)
        .or(get_transactions_route)
        .or(get_transaction_by_id_route)
        .or(vote_route)
}
