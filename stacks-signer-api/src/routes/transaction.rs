use std::convert::Infallible;

use crate::{
    db::{self, paginate_items, vote::get_vote_by_id},
    routes::with_pool,
    transaction::TransactionResponse,
    vote::VoteStatus,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use tracing::error;
use utoipa::IntoParams;
use warp::{hyper::StatusCode, Filter, Reply};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
/// Query parameters for the transaction list
pub struct TransactionQuery {
    /// The page number.
    pub page: Option<usize>,
    /// The limit of signers per page.
    pub limit: Option<usize>,
    /// The transaction status to filter by.
    pub status: Option<VoteStatus>,
}
/// Get transaction by id
#[utoipa::path(
    get,
    path = "/v1/transactions/{id}",
    responses(
        (status = 200, description = "Transaction found successfully", body = TransactionResponse),
        (status = NOT_FOUND, description = "No transaction was found")
    ),
    params(
        ("id" = String, Path, description = "Transaction id for retrieving a specific Transaction"),
    )
)]
async fn get_transaction_by_id(id: String, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    if let Ok(tx) = db::transaction::get_transaction_by_id(id.as_str(), &pool).await {
        if let Ok(vote) = db::vote::get_vote_by_id(id.as_str(), &pool).await {
            let tx_response = TransactionResponse {
                transaction: tx,
                vote_tally: vote.vote_tally,
                vote_choice: vote.vote_choice,
                vote_mechanism: vote.vote_mechanism,
            };
            Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&tx_response),
                StatusCode::OK,
            )))
        } else {
            error!(
                "No vote found for transaction: {}. Database may be corrupted!",
                id
            );
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    } else {
        Ok(Box::new(StatusCode::NOT_FOUND))
    }
}

/// Get list of all transactions
#[utoipa::path(
get,
path = "/v1/transactions",
responses(
    (status = 200, description = "Transaction list returned succesfully", body = Vec<TransactionResponse>),
    (status = INTERNAL_SERVER_ERROR, description = "Internal server error")
),
params(TransactionQuery)
)]
async fn get_transactions(
    query: TransactionQuery,
    pool: SqlitePool,
) -> Result<Box<dyn Reply>, Infallible> {
    let mut filtered_transactions: Vec<TransactionResponse> = vec![];
    if let Ok(txs) = db::transaction::get_transactions(&pool).await {
        for tx in txs {
            if let Ok(vote) = get_vote_by_id(&tx.txid, &pool).await {
                let tx_response = TransactionResponse {
                    transaction: tx,
                    vote_tally: vote.vote_tally,
                    vote_mechanism: vote.vote_mechanism,
                    vote_choice: vote.vote_choice,
                };
                if let Some(status) = query.status {
                    if vote.vote_tally.vote_status == status {
                        filtered_transactions.push(tx_response);
                    }
                } else {
                    filtered_transactions.push(tx_response);
                }
            } else {
                error!(
                    "Could not find cooresponding vote for transaction: {}. Database may be corrupted!",
                    tx.txid
                );
                return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
            }
        }
        let results = paginate_items(&filtered_transactions, query.page, query.limit);
        Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&results),
            StatusCode::OK,
        )))
    } else {
        Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
    }
}

/// Route for getting a list of transactions.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_transactions_route endpoint for routing HTTP requests.
pub fn get_transactions_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("v1" / "transactions"))
        .and(warp::path::end())
        .and(warp::query::<TransactionQuery>())
        .and(with_pool(pool))
        .and_then(get_transactions)
}

/// Route for getting a transaction by ID.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_transaction_by_id_route endpoint for routing HTTP requests.
pub fn get_transaction_by_id_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("v1" / "transactions" / String))
        .and(with_pool(pool))
        .and_then(get_transaction_by_id)
}
