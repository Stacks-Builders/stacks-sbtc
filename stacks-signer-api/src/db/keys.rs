use crate::{
    db::{paginate_items, signers::add_signer, Error},
    key::Key,
    routes::keys::KeysQuery,
    signer::{Signer, Status},
};

use sqlx::{Row, SqlitePool};
use warp::http;

// SQL queries used for performing various operations on the "keys" table.
const SQL_INSERT_KEY: &str =
    "INSERT OR REPLACE INTO keys (signer_id, user_id, key) VALUES (?1, ?2, ?3)";
const SQL_DELETE_KEY: &str = "DELETE FROM keys WHERE signer_id = ?1 AND user_id = ?2 AND key = ?3";
const SQL_DELETE_KEYS_BY_ID: &str = "DELETE FROM keys WHERE signer_id = ?1 AND user_id = ?2";
const SQL_SELECT_KEYS: &str =
    "SELECT key FROM keys WHERE signer_id = ?1 AND user_id = ?2 ORDER BY key ASC";
const SQL_COUNT_KEYS_BY_ID: &str =
    "SELECT COUNT(*) FROM keys WHERE signer_id = ?1 AND user_id = ?2";

/// Add a given delegator key to the database.
///
/// # Params
/// * key: Key - The delegator key to be added.
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection> - The JSON response as a result
/// indicating if the operation was successful or not.
pub async fn add_key(key: Key, pool: SqlitePool) -> Result<impl warp::Reply, warp::Rejection> {
    // First make sure we have an existing signer id
    let count: i64 = sqlx::query(SQL_COUNT_KEYS_BY_ID)
        .bind(key.signer_id)
        .bind(key.user_id)
        .fetch_one(&pool)
        .await
        .map_err(Error::from)?
        .get(0);

    if count == 0 {
        let signer = Signer {
            signer_id: key.signer_id,
            user_id: key.user_id,
            status: Status::Inactive,
        };
        add_signer(signer, pool.clone()).await?;
    }

    // Insert the key into the database
    sqlx::query(SQL_INSERT_KEY)
        .bind(key.signer_id)
        .bind(key.user_id)
        .bind(key.key.as_str())
        .execute(&pool)
        .await
        .map_err(Error::from)?;

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "status": "added" })),
        http::StatusCode::CREATED,
    ))
}

/// Delete all delegator keys for a given signer id and user id.
///
/// # Params
/// * signer_id: i64 - The signer ID.
/// * user_id: i64 - The user ID.
/// * pool: &SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection> - The JSON response as a result,
///   indicating if the operation was successful or not.
pub async fn delete_keys_by_id(
    signer_id: i64,
    user_id: i64,
    pool: &SqlitePool,
) -> Result<impl warp::Reply, warp::Rejection> {
    let rows_deleted = sqlx::query(SQL_DELETE_KEYS_BY_ID)
        .bind(signer_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(Error::from)?
        .rows_affected();
    if rows_deleted == 0 {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({ "error": "not found" })),
            http::StatusCode::NOT_FOUND,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({ "status": "deleted" })),
            http::StatusCode::OK,
        ))
    }
}

/// Delete a given delegator key from the database.
///
/// # Params
/// * key: Key - The delegator key to be deleted.
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection> - The JSON response as a result,
///   indicating if the operation was successful or not.
pub async fn delete_key(key: Key, pool: SqlitePool) -> Result<impl warp::Reply, warp::Rejection> {
    let rows_deleted = sqlx::query(SQL_DELETE_KEY)
        .bind(key.signer_id)
        .bind(key.user_id)
        .bind(key.key.as_str())
        .execute(&pool)
        .await
        .map_err(Error::from)?
        .rows_affected();
    if rows_deleted == 0 {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({ "error": "not found" })),
            http::StatusCode::NOT_FOUND,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({ "status": "deleted" })),
            http::StatusCode::OK,
        ))
    }
}

/// Get all delegator keys for a given signer id and user id.
///
/// # Params
/// * query: KeysQuery - Query parameters specifying the signer ID, user ID,
///   and optional pagination settings.
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection> - The JSON response as a result,
///   containing the list of delegator keys.
pub async fn get_keys(
    query: KeysQuery,
    pool: SqlitePool,
) -> Result<impl warp::Reply, warp::Rejection> {
    let keys: Vec<String> = sqlx::query(SQL_SELECT_KEYS)
        .bind(query.signer_id)
        .bind(query.user_id)
        .fetch_all(&pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|row: &sqlx::sqlite::SqliteRow| row.get(0))
        .collect();

    let displayed_keys = paginate_items(&keys, query.page, query.limit);
    let json_response = warp::reply::with_status(
        warp::reply::json(&serde_json::json!(displayed_keys)),
        http::StatusCode::OK,
    );
    Ok(json_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_pool;
    use warp::http::StatusCode;
    use warp::Reply;

    async fn init_db() -> SqlitePool {
        init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_key() {
        let pool = init_db().await;
        let key = Key {
            signer_id: 1,
            user_id: 1,
            key: "key".to_string(),
        };

        let response = add_key(key.clone(), pool.clone())
            .await
            .expect("failed to add key");
        assert_eq!(response.into_response().status(), StatusCode::CREATED);

        let row = sqlx::query(
                "SELECT signer_id, user_id, key FROM keys WHERE signer_id = ?1 AND user_id = ?2 AND key = ?3").bind(key.signer_id).bind(key.user_id).bind(key.key.as_str()).fetch_one(&pool).await.expect("Failed to get added key");
        assert_eq!(
            (row.get(0), row.get(1), row.get(2)),
            (key.signer_id, key.user_id, key.key)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_key() {
        let pool = init_db().await;
        let key = Key {
            signer_id: 1,
            user_id: 1,
            key: "key".to_string(),
        };

        add_key(key.clone(), pool.clone())
            .await
            .expect("failed to add key");

        let response = delete_key(key.clone(), pool.clone())
            .await
            .expect("failed to delete key");
        assert_eq!(response.into_response().status(), StatusCode::OK);

        let row_count: i64 = sqlx::query(
            "SELECT COUNT(*) FROM keys WHERE signer_id = ?1 AND user_id = ?2 AND key = ?3",
        )
        .bind(key.signer_id)
        .bind(key.user_id)
        .bind(key.key)
        .fetch_one(&pool)
        .await
        .expect("Failed to get number of keys")
        .get(0);
        assert_eq!(row_count, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_keys() {
        let pool = init_db().await;
        let keys_to_insert = vec![
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key1".to_string(),
            },
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key2".to_string(),
            },
            Key {
                signer_id: 2,
                user_id: 1,
                key: "key3".to_string(),
            },
            Key {
                signer_id: 2,
                user_id: 1,
                key: "key1".to_string(),
            },
        ];

        for key in keys_to_insert.clone() {
            add_key(key, pool.clone()).await.expect("failed to add key");
        }

        let query = KeysQuery {
            signer_id: 1,
            user_id: 1,
            page: Some(1),
            limit: Some(2),
        };
        let body = get_keys(query.clone(), pool.clone())
            .await
            .expect("failed to get keys")
            .into_response()
            .into_body();
        let body_bytes = warp::hyper::body::to_bytes(body)
            .await
            .expect("failed to get response bytes");

        let keys: Vec<String> =
            serde_json::from_slice(&body_bytes).expect("failed to deserialize keys");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys, vec!["key1", "key2"]);

        let query = KeysQuery {
            signer_id: 2,
            user_id: 1,
            page: Some(1),
            limit: Some(2),
        };

        let body = get_keys(query.clone(), pool.clone())
            .await
            .expect("failed to get keys")
            .into_response()
            .into_body();
        let body_bytes = warp::hyper::body::to_bytes(body)
            .await
            .expect("failed to get response bytes");

        let keys: Vec<String> =
            serde_json::from_slice(&body_bytes).expect("failed to deserialize keys");
        assert_eq!(keys, vec!["key1", "key3"]);

        let query = KeysQuery {
            signer_id: 2,
            user_id: 1,
            page: Some(1),
            limit: Some(1),
        };

        let body = get_keys(query.clone(), pool.clone())
            .await
            .expect("failed to get keys")
            .into_response()
            .into_body();
        let body_bytes = warp::hyper::body::to_bytes(body)
            .await
            .expect("failed to get response bytes");

        let keys: Vec<String> =
            serde_json::from_slice(&body_bytes).expect("failed to deserialize keys");
        assert_eq!(keys, vec!["key1"]);
    }
}
