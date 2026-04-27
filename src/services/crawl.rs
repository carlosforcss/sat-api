use std::sync::{Arc, OnceLock};

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::repositories::crawl::Crawl;
use crate::repositories::{crawl, link};
use crate::storage::S3Storage;

const MAX_CONCURRENT_CRAWLS: usize = 3;
static CRAWL_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn crawl_semaphore() -> Arc<Semaphore> {
    CRAWL_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_CRAWLS)))
        .clone()
}

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
    storage: Arc<S3Storage>,
    user_id: i32,
    link_id: i32,
    crawl_type: &str,
    params: serde_json::Value,
) -> Result<Crawl, CrawlError> {
    link::find_by_id_and_user(pool, link_id, user_id)
        .await
        .map_err(|_| CrawlError::Internal)?
        .ok_or(CrawlError::NotFound)?;

    let crawl = crawl::create(pool, user_id, link_id, crawl_type, params)
        .await
        .map_err(|e| {
            tracing::error!("failed to create crawl: {e}");
            CrawlError::Internal
        })?;

    spawn(pool, crawl.id, storage);

    Ok(crawl)
}

pub fn spawn(pool: &PgPool, crawl_id: i32, storage: Arc<S3Storage>) {
    let pool_clone = pool.clone();
    let sem = crawl_semaphore();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build crawler runtime");
        rt.block_on(async move {
            let _permit = sem.acquire_owned().await.expect("crawl semaphore closed");
            crate::crawlers::run_crawl(&pool_clone, crawl_id, storage).await;
        });
    });
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: CrawlFilters,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Crawl>, i64), CrawlError> {
    let (_, per_page, offset) = crate::services::paginate(page, per_page);
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
