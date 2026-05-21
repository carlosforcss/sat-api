use sqlx::PgPool;

use crate::error::ApiError;
use crate::repositories::link;
use crate::repositories::link::Link;

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    per_page: i64,
    offset: i64,
) -> Result<(Vec<Link>, i64), ApiError> {
    link::list_by_user(pool, user_id, per_page, offset)
        .await
        .map_err(|_| ApiError::Internal)
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, ApiError> {
    link::delete(pool, id, user_id).await.map_err(|e| {
        if crate::repositories::is_fk_violation(&e) {
            ApiError::Conflict("link cannot be deleted")
        } else {
            ApiError::Internal
        }
    })
}
