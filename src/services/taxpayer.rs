use sqlx::PgPool;

use crate::error::ApiError;
use crate::repositories::taxpayer::{self, Taxpayer, TaxpayerFilters};

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: TaxpayerFilters,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<Taxpayer>, i64), ApiError> {
    taxpayer::list_for_user(pool, user_id, filters, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("failed to list taxpayers: {e}");
            ApiError::Internal
        })
}

pub async fn get(pool: &PgPool, user_id: i32, id: i32) -> Result<Taxpayer, ApiError> {
    taxpayer::find_by_id_for_user(pool, id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch taxpayer {id}: {e}");
            ApiError::Internal
        })?
        .ok_or(ApiError::NotFound("not found"))
}
