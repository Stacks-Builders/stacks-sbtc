use crate::{
    db::signers::{add_signer, delete_signer, get_signers},
    routes::{json_body, with_pool},
    signer::{Signer, Status},
};
use serde::Deserialize;
use sqlx::SqlitePool;
use warp::Filter;

#[derive(Debug, Deserialize, Clone)]
/// Query parameters for the get signers route.
pub struct SignerQuery {
    /// The current status of the signers to retrieve.
    pub status: Option<Status>,
    /// The page number.
    pub page: Option<usize>,
    /// The limit of signers per page.
    pub limit: Option<usize>,
}

/// Route for adding a signer.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the add_signer_route endpoint for routing HTTP requests.
pub fn add_signer_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("v1"))
        .and(warp::path("signers"))
        .and(warp::path::end())
        .and(json_body::<Signer>())
        .and(with_pool(pool))
        .and_then(add_signer)
}

/// Route for deleting a signer.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the delete_signer_route endpoint for routing HTTP requests.
pub fn delete_signer_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::delete()
        .and(warp::path("v1"))
        .and(warp::path("signers"))
        .and(warp::path::end())
        .and(json_body::<Signer>())
        .and(with_pool(pool))
        .and_then(delete_signer)
}

/// Route for fetching all signers.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_signers_route endpoint for routing HTTP requests.
// TODO: update this to only retrieve the signers for a given user
pub fn get_signers_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("v1"))
        .and(warp::path("signers"))
        .and(warp::query::<SignerQuery>())
        .and(warp::path::end())
        .and(with_pool(pool))
        .and_then(get_signers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_pool, signers::add_signer};
    use crate::signer::{Signer, Status};
    use warp::http;

    async fn init_db() -> SqlitePool {
        let pool = init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.");
        insert_test_data(pool.clone()).await;
        pool
    }

    // Insert test data into the database
    async fn insert_test_data(pool: SqlitePool) {
        add_signer(
            Signer {
                signer_id: 1,
                user_id: 2,
                status: Status::Active,
            },
            pool.clone(),
        )
        .await
        .unwrap();
        add_signer(
            Signer {
                signer_id: 3,
                user_id: 4,
                status: Status::Inactive,
            },
            pool,
        )
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_signers() {
        let pool = init_db().await;

        // Add test data
        insert_test_data(pool.clone()).await;

        let api = warp::test::request()
            .path("/v1/signers?status=active")
            .method("GET")
            .header("content-type", "application/json")
            .reply(&get_signers_route(pool))
            .await;
        let body = api.body();

        let signers: Vec<Signer> =
            serde_json::from_slice(&body).expect("failed to deserialize Signers");
        assert_eq!(signers.len(), 1);
        assert_eq!(
            signers[0],
            Signer {
                signer_id: 1,
                user_id: 2,
                status: Status::Active,
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_signer() {
        let pool = init_db().await;

        let new_signer = Signer {
            signer_id: 5,
            user_id: 6,
            status: Status::Active,
        };

        let api = warp::test::request()
            .path("/v1/signers")
            .method("POST")
            .json(&new_signer)
            .reply(&add_signer_route(pool))
            .await;

        assert_eq!(api.status(), http::StatusCode::CREATED);
        assert_eq!(api.body(), r#"{"status":"added"}"#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_signer() {
        let pool = init_db().await;

        // Add test data
        insert_test_data(pool.clone()).await;

        let signer_to_delete = Signer {
            signer_id: 1,
            user_id: 2,
            status: Status::Active,
        };

        let api = warp::test::request()
            .path("/v1/signers")
            .method("DELETE")
            .header("content-type", "application/json")
            .json(&signer_to_delete)
            .reply(&delete_signer_route(pool))
            .await;

        assert_eq!(api.status(), http::StatusCode::OK);
        assert_eq!(api.body(), r#"{"status":"deleted"}"#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_signer_not_found() {
        let pool = init_db().await;

        // Add test data
        insert_test_data(pool.clone()).await;

        let signer_to_delete = Signer {
            signer_id: 5,
            user_id: 2,
            status: Status::Active,
        };

        let api = warp::test::request()
            .path("/v1/signers")
            .method("DELETE")
            .header("content-type", "application/json")
            .json(&signer_to_delete)
            .reply(&delete_signer_route(pool))
            .await;

        assert_eq!(api.status(), http::StatusCode::NOT_FOUND);
        assert_eq!(api.body(), r#"{"error":"not found"}"#);
    }
}
