use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::crawl::{self, Crawl};

pub enum CrawlError {
    NotFound,
    Internal,
}

impl IntoResponse for CrawlError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            CrawlError::NotFound => (StatusCode::NOT_FOUND, "crawl not found"),
            CrawlError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub struct CrawlFilters {
    pub credential_id: Option<i32>,
    pub crawl_type: Option<String>,
    pub status: Option<String>,
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: CrawlFilters,
) -> Result<Vec<Crawl>, CrawlError> {
    crawl::list_for_user(
        pool,
        user_id,
        filters.credential_id,
        filters.crawl_type.as_deref(),
        filters.status.as_deref(),
    )
    .await
    .map_err(|_| CrawlError::Internal)
}

pub async fn get(pool: &PgPool, crawl_id: i32, user_id: i32) -> Result<Crawl, CrawlError> {
    crawl::find_by_id_for_user(pool, crawl_id, user_id)
        .await
        .map_err(|_| CrawlError::Internal)?
        .ok_or(CrawlError::NotFound)
}
