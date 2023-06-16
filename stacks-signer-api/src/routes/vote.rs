use std::convert::Infallible;

use crate::{
    db::vote::{add_vote, get_vote_by_id},
    routes::{json_body, with_pool},
    vote::{VoteRequest, VoteResponse, VoteStatus},
};
use sqlx::SqlitePool;
use warp::{hyper::StatusCode, Filter, Reply};

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
    if let Ok(mut vote) = get_vote_by_id(&vote_request.txid, &pool).await {
        let vote_choice = vote_request.vote_choice;
        if vote.vote_choice.is_some() {
            return Ok(Box::new(StatusCode::CONFLICT));
        }
        if vote.vote_tally.vote_status != VoteStatus::Pending {
            return Ok(Box::new(StatusCode::FORBIDDEN));
        }
        vote.vote_choice = Some(vote_choice);
        // TODO: update consensus correctly
        vote.vote_tally.current_consensus += 1;
        if add_vote(&vote, &pool).await.is_ok() {
            Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&VoteResponse {
                    vote_choice,
                    vote_tally: vote.vote_tally,
                }),
                StatusCode::OK,
            )))
        } else {
            Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    } else {
        Ok(Box::new(StatusCode::NOT_FOUND))
    }
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
