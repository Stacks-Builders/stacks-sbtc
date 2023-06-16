use std::convert::Infallible;

use crate::{db::paginate_items, transaction::create_dummy_transactons, vote::VoteStatus};
use serde::Deserialize;
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
        ("id" = usize, Path, description = "Transaction id for retrieving a specific Transaction"),
    )
)]
async fn get_transaction_by_id(id: String) -> Result<Box<dyn Reply>, Infallible> {
    let txs = create_dummy_transactons();
    if let Ok(id) = id.parse::<usize>() {
        if id >= txs.len() {
            return Ok(Box::new(StatusCode::NOT_FOUND));
        }
        return Ok(Box::new(warp::reply::with_status(
            warp::reply::json(&txs[id]),
            StatusCode::OK,
        )));
    }
    return Ok(Box::new(StatusCode::NOT_FOUND));
}

/// Get list of all transactions
#[utoipa::path(
get,
path = "/v1/transactions",
responses(
    (status = 200, description = "Transaction list returned succesfully", body = Vec<TransactionResponse>)
),
params(TransactionQuery)
)]
async fn get_transactions(query: TransactionQuery) -> Result<impl Reply, Infallible> {
    let mut filtered_transactions = vec![];
    let transactions = create_dummy_transactons();
    if let Some(status) = query.status {
        transactions.into_iter().for_each(|tx| {
            if tx.vote_tally.vote_status == status {
                filtered_transactions.push(tx);
            }
        });
    } else {
        filtered_transactions = transactions;
    }
    let results = paginate_items(&filtered_transactions, query.page, query.limit);
    Ok(Box::new(warp::reply::with_status(
        warp::reply::json(&results),
        StatusCode::OK,
    )))
}

/// Route for getting a list of transactions.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_transactions_route endpoint for routing HTTP requests.
pub fn get_transactions_route(
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("v1" / "transactions"))
        .and(warp::path::end())
        .and(warp::query::<TransactionQuery>())
        .and_then(get_transactions)
}

/// Route for getting a transaction by ID.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_transaction_by_id_route endpoint for routing HTTP requests.
pub fn get_transaction_by_id_route(
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("v1" / "transactions" / String))
        .and_then(get_transaction_by_id)
}
