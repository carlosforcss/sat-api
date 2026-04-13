use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::link;
use crate::repositories::link::Link;

pub enum LinkError {
    Internal,
}

impl IntoResponse for LinkError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "internal error" })),
        )
            .into_response()
    }
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Link>, i64), LinkError> {
    let per_page = per_page.clamp(1, 100);
    let page = page.max(1);
    let offset = (page - 1) * per_page;
    link::list_by_user(pool, user_id, per_page, offset)
        .await
        .map_err(|_| LinkError::Internal)
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, LinkError> {
    link::delete(pool, id, user_id)
        .await
        .map_err(|_| LinkError::Internal)
}
