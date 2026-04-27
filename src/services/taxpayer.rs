use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::taxpayer::{self, Taxpayer, TaxpayerFilters};

pub enum TaxpayerError {
    Internal,
    NotFound,
}

impl IntoResponse for TaxpayerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            TaxpayerError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal error" })),
            )
                .into_response(),
            TaxpayerError::NotFound => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response()
            }
        }
    }
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: TaxpayerFilters,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Taxpayer>, i64), TaxpayerError> {
    let (_, per_page, offset) = crate::services::paginate(page, per_page);
    taxpayer::list_for_user(pool, user_id, filters, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("failed to list taxpayers: {e}");
            TaxpayerError::Internal
        })
}

pub async fn get(pool: &PgPool, user_id: i32, id: i32) -> Result<Taxpayer, TaxpayerError> {
    taxpayer::find_by_id_for_user(pool, id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch taxpayer {id}: {e}");
            TaxpayerError::Internal
        })?
        .ok_or(TaxpayerError::NotFound)
}
