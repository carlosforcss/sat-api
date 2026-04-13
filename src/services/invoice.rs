use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::invoice::{self, Invoice, InvoiceFilters};

pub enum InvoiceError {
    Internal,
}

impl IntoResponse for InvoiceError {
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
    filters: InvoiceFilters,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Invoice>, i64), InvoiceError> {
    let per_page = per_page.clamp(1, 100);
    let page = page.max(1);
    let offset = (page - 1) * per_page;
    invoice::list_for_user(pool, user_id, filters, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("failed to list invoices: {e}");
            InvoiceError::Internal
        })
}
