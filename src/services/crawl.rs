use std::sync::{Arc, OnceLock};

use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::error::ApiError;
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
) -> Result<Crawl, ApiError> {
    link::find_by_id_and_user(pool, link_id, user_id)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::NotFound("crawl not found"))?;

    let crawl = crawl::create(pool, user_id, link_id, crawl_type, params)
        .await
        .map_err(|e| {
            tracing::error!("failed to create crawl: {e}");
            ApiError::Internal
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
    per_page: i64,
    offset: i64,
) -> Result<(Vec<Crawl>, i64), ApiError> {
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
    .map_err(|_| ApiError::Internal)
}

pub async fn get(pool: &PgPool, crawl_id: i32, user_id: i32) -> Result<Crawl, ApiError> {
    crawl::find_by_id_for_user(pool, crawl_id, user_id)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::NotFound("crawl not found"))
}
