use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::link;
use crate::repositories::link::Link;

pub enum LinkError {
    Internal,
    InUse,
}

impl IntoResponse for LinkError {
    fn into_response(self) -> axum::response::Response {
        match self {
            LinkError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal error" })),
            )
                .into_response(),
            LinkError::InUse => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "link cannot be deleted" })),
            )
                .into_response(),
        }
    }
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Link>, i64), LinkError> {
    let (_, per_page, offset) = crate::services::paginate(page, per_page);
    link::list_by_user(pool, user_id, per_page, offset)
        .await
        .map_err(|_| LinkError::Internal)
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, LinkError> {
    link::delete(pool, id, user_id).await.map_err(|e| {
        if crate::repositories::is_fk_violation(&e) {
            LinkError::InUse
        } else {
            LinkError::Internal
        }
    })
}
