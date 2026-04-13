use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::invoice::{self, Invoice, InvoiceFilters};

pub enum InvoiceError {
    Internal,
    NotFound,
}

impl IntoResponse for InvoiceError {
    fn into_response(self) -> axum::response::Response {
        match self {
            InvoiceError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal error" })),
            )
                .into_response(),
            InvoiceError::NotFound => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response()
            }
        }
    }
}

pub async fn get_invoice_file(
    pool: &PgPool,
    user_id: i32,
    invoice_id: i32,
    extension: &str,
) -> Result<(Vec<u8>, String), InvoiceError> {
    let inv = invoice::find_by_id_for_user(pool, invoice_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch invoice {invoice_id}: {e}");
            InvoiceError::Internal
        })?
        .ok_or(InvoiceError::NotFound)?;

    let path = std::path::Path::new(&inv.download_path).join(format!("{}.{}", inv.uuid, extension));

    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        tracing::error!("failed to read invoice file {:?}: {e}", path);
        InvoiceError::NotFound
    })?;

    Ok((bytes, inv.uuid))
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
