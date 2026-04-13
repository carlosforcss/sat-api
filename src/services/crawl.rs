use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::crawl::Crawl;
use crate::repositories::{crawl, link};

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
    pub link_id: Option<i32>,
    pub crawl_type: Option<String>,
    pub status: Option<String>,
}

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    link_id: i32,
    crawl_type: &str,
    params: serde_json::Value,
) -> Result<Crawl, CrawlError> {
    link::find_by_id_and_user(pool, link_id, user_id)
        .await
        .map_err(|_| CrawlError::Internal)?
        .ok_or(CrawlError::NotFound)?;

    let crawl = crawl::create(pool, link_id, crawl_type, params)
        .await
        .map_err(|e| {
            tracing::error!("failed to create crawl: {e}");
            CrawlError::Internal
        })?;

    spawn(pool, crawl.id);

    Ok(crawl)
}

pub fn spawn(pool: &PgPool, crawl_id: i32) {
    let pool_clone = pool.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build crawler runtime");
        rt.block_on(crate::crawlers::run_crawl(&pool_clone, crawl_id));
    });
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: CrawlFilters,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Crawl>, i64), CrawlError> {
    let per_page = per_page.clamp(1, 100);
    let page = page.max(1);
    let offset = (page - 1) * per_page;
    crawl::list_for_user(
        pool,
        user_id,
        filters.link_id,
        filters.crawl_type.as_deref(),
        filters.status.as_deref(),
        per_page,
        offset,
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
