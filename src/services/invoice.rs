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
) -> Result<Vec<Invoice>, InvoiceError> {
    invoice::list_for_user(pool, user_id, filters)
        .await
        .map_err(|e| {
            tracing::error!("failed to list invoices: {e}");
            InvoiceError::Internal
        })
}
