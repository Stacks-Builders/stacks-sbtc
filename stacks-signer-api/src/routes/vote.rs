use std::convert::Infallible;

use crate::{
    db::paginate_items,
    routes::{json_body, with_pool},
    transaction::create_dummy_transactons,
    vote::{VoteRequest, VoteStatus},
};
use serde::Deserialize;
use sqlx::SqlitePool;
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

/// Vote for a transaction
#[utoipa::path(
    post,
    path = "/v1/vote",
    request_body = VoteRequest,
    responses(
        (status = OK, description = "Vote was cast.", body = VoteResponse),
        (status = NOT_FOUND, description = "Requested transaction not found."),
        (status = CONFLICT, description = "Vote has already been cast."),
        (status = BAD_REQUEST, description = "Invalid vote."),
        (status = FORBIDDEN, description = "Voting period has ended.")
    )
)]
async fn vote(vote_request: VoteRequest, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    Ok(Box::new(StatusCode::OK))
}

/// Route for voting to approve or reject a specific transaction.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_transactions_route endpoint for routing HTTP requests.
pub fn vote_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path!("v1" / "vote"))
        .and(warp::path::end())
        .and(json_body::<VoteRequest>())
        .and(with_pool(pool))
        .and_then(vote) 
}
