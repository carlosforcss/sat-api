use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::{credential, link};
use crate::repositories::link::Link;

pub enum LinkError {
    NotFound,
    Internal,
}

impl IntoResponse for LinkError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            LinkError::NotFound => (StatusCode::NOT_FOUND, "not found"),
            LinkError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub async fn create(pool: &PgPool, user_id: i32, credential_id: i32) -> Result<Link, LinkError> {
    let cred = credential::find_by_id_and_user(pool, credential_id, user_id)
        .await
        .map_err(|_| LinkError::Internal)?
        .ok_or(LinkError::NotFound)?;

    let lnk = link::create(pool, user_id, credential_id, &cred.taxpayer_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to create link: {e}");
            LinkError::Internal
        })?;

    spawn_validate_crawl(pool, lnk.id).await;

    Ok(lnk)
}

pub async fn list(pool: &PgPool, user_id: i32) -> Result<Vec<Link>, LinkError> {
    link::list_by_user(pool, user_id)
        .await
        .map_err(|_| LinkError::Internal)
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, LinkError> {
    link::delete(pool, id, user_id)
        .await
        .map_err(|_| LinkError::Internal)
}

async fn spawn_validate_crawl(pool: &PgPool, link_id: i32) {
    match crate::repositories::crawl::create(
        pool,
        link_id,
        "VALIDATE_CREDENTIALS",
        serde_json::json!({}),
    )
    .await
    {
        Ok(crawl) => crate::services::crawl::spawn(pool, crawl.id),
        Err(e) => {
            tracing::error!("failed to create validation crawl for link {link_id}: {e}");
        }
    }
}
