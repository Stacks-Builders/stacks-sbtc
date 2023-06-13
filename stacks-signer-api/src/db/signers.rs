use crate::{
    db::{keys::delete_keys_by_id, paginate_items, Error},
    routes::signers::SignerQuery,
    signer::Signer,
};

use sqlx::{Row, SqlitePool};
use warp::http;
// SQL constants for interacting with the SQLite database.
const SQL_INSERT_SIGNER: &str =
    "INSERT OR REPLACE INTO sbtc_signers (signer_id, user_id, status) VALUES (?1, ?2, ?3)";
const SQL_DELETE_SIGNER: &str = "DELETE FROM sbtc_signers WHERE signer_id = ?1 AND user_id = ?2";
const SQL_SELECT_SIGNER: &str =
    "SELECT signer_id, user_id, status FROM sbtc_signers ORDER BY signer_id ASC";
const SQL_SELECT_SIGNER_BY_STATUS: &str =
    "SELECT signer_id, user_id, status FROM sbtc_signers WHERE status = ?1 ORDER BY signer_id ASC";

/// Add a given signer to the database.
///
/// # Params
/// * signer: Signer - The signer object to add to the database.
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection>: A Warp reply containing the HTTP response with the result.
pub async fn add_signer(
    signer: Signer,
    pool: SqlitePool,
) -> Result<impl warp::Reply, warp::Rejection> {
    sqlx::query(SQL_INSERT_SIGNER)
        .bind(signer.signer_id)
        .bind(signer.user_id)
        .bind(signer.status.as_str())
        .execute(&pool)
        .await
        .map_err(Error::from)?;

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "status": "added" })),
        http::StatusCode::CREATED,
    ))
}

/// Delete a given signer from the database.
///
/// # Params
/// * signer: Signer - The signer object to delete from the database.
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection>: A Warp reply containing the HTTP response with the result.
pub async fn delete_signer(
    signer: Signer,
    pool: SqlitePool,
) -> Result<impl warp::Reply, warp::Rejection> {
    // First delete any corresponding keys
    delete_keys_by_id(signer.signer_id, signer.user_id, &pool).await?;

    let rows_deleted = sqlx::query(SQL_DELETE_SIGNER)
        .bind(signer.signer_id)
        .bind(signer.user_id)
        .execute(&pool)
        .await
        .map_err(Error::from)?
        .rows_affected();
    // Delete the signer itself
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

/// Get a list of signers from the database given an optional status.
///
/// # Params
/// * query: SignerQuery - A query object containing the optional status filter and pagination options.
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
///
/// # Returns
/// * Result<impl warp::Reply, warp::Rejection>: A Warp reply containing the HTTP response with the list of signers.
pub async fn get_signers(
    query: SignerQuery,
    pool: SqlitePool,
) -> Result<impl warp::Reply, warp::Rejection> {
    let sqlite_query = if let Some(status) = query.status.map(|s| s.as_str()) {
        sqlx::query(SQL_SELECT_SIGNER_BY_STATUS).bind(status)
    } else {
        sqlx::query(SQL_SELECT_SIGNER)
    };
    let signers: Vec<(i64, i64, String)> = sqlite_query
        .fetch_all(&pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|row: &sqlx::sqlite::SqliteRow| (row.get(0), row.get(1), row.get(2)))
        .collect();
    let displayed_signers = paginate_items(&signers, query.page, query.limit);

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!(displayed_signers)),
        http::StatusCode::OK,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::init_pool, signer::Status};
    use warp::http::StatusCode;
    use warp::Reply;

    async fn init_db() -> SqlitePool {
        init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_signer() {
        let pool = init_db().await;
        let signer = Signer {
            signer_id: 1,
            user_id: 1,
            status: Status::Active,
        };

        let response = add_signer(signer.clone(), pool.clone())
            .await
            .expect("failed to add signer");
        assert_eq!(response.into_response().status(), StatusCode::CREATED);

        let row = sqlx::query(SQL_SELECT_SIGNER)
            .bind(signer.signer_id)
            .bind(signer.user_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to get added signer");
        assert_eq!(
            (row.get(0), row.get(1), row.get(2)),
            (
                signer.signer_id,
                signer.user_id,
                signer.status.as_str().to_string()
            )
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_signer() {
        let pool = init_db().await;
        let signer = Signer {
            signer_id: 1,
            user_id: 1,
            status: Status::Active,
        };

        add_signer(signer, pool.clone())
            .await
            .expect("failed to add signer");

        let response = delete_signer(signer.clone(), pool.clone())
            .await
            .expect("failed to delete signer");
        assert_eq!(response.into_response().status(), StatusCode::OK);

        let row_count: i64 =
            sqlx::query("SELECT COUNT(*) FROM sbtc_signers WHERE signer_id = ?1 AND user_id = ?2")
                .bind(signer.signer_id)
                .bind(signer.user_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to get row count")
                .get(0);
        assert_eq!(row_count, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_signers() {
        let pool = init_db().await;
        let signers_to_insert = vec![
            Signer {
                signer_id: 1,
                user_id: 1,
                status: Status::Active,
            },
            Signer {
                signer_id: 2,
                user_id: 2,
                status: Status::Active,
            },
            Signer {
                signer_id: 3,
                user_id: 3,
                status: Status::Inactive,
            },
        ];

        for signer in signers_to_insert.clone() {
            add_signer(signer, pool.clone())
                .await
                .expect("failed to add signer");
        }

        let query = SignerQuery {
            page: Some(1),
            limit: Some(2),
            status: Some(Status::Active),
        };
        let body = get_signers(query.clone(), pool.clone())
            .await
            .expect("failed to get signers")
            .into_response()
            .into_body();
        let body_bytes = warp::hyper::body::to_bytes(body)
            .await
            .expect("failed to get response bytes");

        let signers: Vec<Signer> =
            serde_json::from_slice(&body_bytes).expect("failed to deserialize signers");
        assert_eq!(signers, &signers_to_insert[0..2]);

        let query = SignerQuery {
            page: Some(2),
            limit: Some(2),
            status: None,
        };

        let body = get_signers(query.clone(), pool.clone())
            .await
            .expect("failed to get signers")
            .into_response()
            .into_body();
        let body_bytes = warp::hyper::body::to_bytes(body)
            .await
            .expect("failed to get response bytes");

        let signers: Vec<Signer> =
            serde_json::from_slice(&body_bytes).expect("failed to deserialize signers");
        assert_eq!(signers, &signers_to_insert[2..3]);

        let query = SignerQuery {
            page: None,
            limit: None,
            status: None,
        };

        let body = get_signers(query.clone(), pool.clone())
            .await
            .expect("failed to get signers")
            .into_response()
            .into_body();
        let body_bytes = warp::hyper::body::to_bytes(body)
            .await
            .expect("failed to get response bytes");

        let signers: Vec<Signer> =
            serde_json::from_slice(&body_bytes).expect("failed to deserialize signers");
        assert_eq!(signers, signers_to_insert);
    }
}
