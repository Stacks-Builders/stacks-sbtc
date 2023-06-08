use crate::{
    db::keys::{add_key, delete_key, get_keys},
    key::Key,
    routes::{json_body, with_pool},
};
use serde::Deserialize;
use sqlx::SqlitePool;
use warp::Filter;

#[derive(Debug, Deserialize, Clone)]
/// The query parameters for the get keys route.
pub struct KeysQuery {
    /// The signer's ID.
    pub signer_id: i64,
    /// The user's ID.
    pub user_id: i64,
    /// The page number.
    pub page: Option<usize>,
    /// The limit of keys per page.
    pub limit: Option<usize>,
}

/// Route for adding a delegator key.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the add_key_route endpoint for routing HTTP requests.
pub fn add_key_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("v1"))
        .and(warp::path("keys"))
        .and(warp::path::end())
        .and(json_body::<Key>())
        .and(with_pool(pool))
        .and_then(add_key)
}

/// Route for deleting a key.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the delete_key_route endpoint for routing HTTP requests.
pub fn delete_key_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::delete()
        .and(warp::path("v1"))
        .and(warp::path("keys"))
        .and(warp::path::end())
        .and(json_body::<Key>())
        .and(with_pool(pool))
        .and_then(delete_key)
}

/// Route for fetching keys.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_keys_route endpoint for routing HTTP requests.
pub fn get_keys_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path("v1"))
        .and(warp::path("keys"))
        .and(warp::query::<KeysQuery>())
        .and(warp::path::end())
        .and(with_pool(pool))
        .and_then(get_keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_pool;
    use warp::http::StatusCode;

    async fn init_db() -> SqlitePool {
        let pool = init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.");
        insert_test_data(pool.clone()).await;
        pool
    }

    // Insert test data into the database
    async fn insert_test_data(pool: SqlitePool) {
        let key1 = Key {
            signer_id: 1,
            user_id: 1,
            key: "key1".to_string(),
        };
        let key2 = Key {
            signer_id: 1,
            user_id: 1,
            key: "key2".to_string(),
        };
        let key3 = Key {
            signer_id: 1,
            user_id: 1,
            key: "key3".to_string(),
        };
        let key4 = Key {
            signer_id: 1,
            user_id: 1,
            key: "key4".to_string(),
        };
        let key5 = Key {
            signer_id: 1,
            user_id: 1,
            key: "key5".to_string(),
        };
        let key_diff_signer_id = Key {
            signer_id: 10,
            user_id: 1,
            key: "key1".to_string(),
        };
        // Add test data
        add_key(key1, pool.clone()).await.unwrap();
        add_key(key2, pool.clone()).await.unwrap();
        add_key(key3, pool.clone()).await.unwrap();
        add_key(key4, pool.clone()).await.unwrap();
        add_key(key5, pool.clone()).await.unwrap();
        add_key(key_diff_signer_id, pool.clone()).await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_keys() {
        let pool = init_db().await;

        // Add test data
        insert_test_data(pool.clone()).await;

        let api = warp::test::request()
            .path("/v1/keys?signer_id=1&user_id=1")
            .method("GET")
            .header("content-type", "application/json")
            .reply(&get_keys_route(pool))
            .await;

        assert_eq!(api.status(), StatusCode::OK);
        assert_eq!(api.body(), r#"["key1","key2","key3","key4","key5"]"#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_key() {
        let pool = init_db().await;

        let new_key = Key {
            signer_id: 1,
            user_id: 1,
            key: "key6".to_string(),
        };

        let api = warp::test::request()
            .path("/v1/keys")
            .method("POST")
            .json(&new_key)
            .reply(&add_key_route(pool))
            .await;

        assert_eq!(api.status(), StatusCode::CREATED);
        assert_eq!(api.body(), r#"{"status":"added"}"#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_key() {
        let pool = init_db().await;

        // Add test data
        insert_test_data(pool.clone()).await;

        let key_to_delete = Key {
            signer_id: 1,
            user_id: 1,
            key: "key1".to_string(),
        };

        let api = warp::test::request()
            .path("/v1/keys")
            .method("DELETE")
            .header("content-type", "application/json")
            .json(&key_to_delete)
            .reply(&delete_key_route(pool))
            .await;

        assert_eq!(api.status(), StatusCode::OK);
        assert_eq!(api.body(), r#"{"status":"deleted"}"#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_key_not_found() {
        let pool = init_db().await;

        // Add test data
        insert_test_data(pool.clone()).await;

        let key_no_matching_user = Key {
            signer_id: 1,
            user_id: 2, // We don't have this user id
            key: "key1".to_string(),
        };

        let key_no_matching_key = Key {
            signer_id: 1,
            user_id: 1,
            key: "invalid key".to_string(),
        };

        let key_no_matching_signer = Key {
            signer_id: 2, // We don't have this signer id
            user_id: 1,
            key: "key1".to_string(),
        };

        let keys_to_attempt = vec![
            key_no_matching_user,
            key_no_matching_key,
            key_no_matching_signer,
        ];

        for key in keys_to_attempt {
            let api = warp::test::request()
                .path("/v1/keys")
                .method("DELETE")
                .header("content-type", "application/json")
                .json(&key)
                .reply(&delete_key_route(pool.clone()))
                .await;

            assert_eq!(api.status(), StatusCode::NOT_FOUND);
            assert_eq!(api.body(), r#"{"error":"not found"}"#);
        }
    }
}
