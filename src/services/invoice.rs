use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::files as files_repo;
use crate::repositories::invoice::{self, Invoice, InvoiceFilters};
use crate::storage::S3Storage;

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
    storage: Arc<S3Storage>,
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

    let file_id = if extension == "xml" {
        inv.xml_file_id
    } else {
        inv.pdf_file_id
    };

    if let Some(id) = file_id {
        let file = files_repo::find_by_id(pool, id)
            .await
            .map_err(|e| {
                tracing::error!("failed to fetch file record {id}: {e}");
                InvoiceError::Internal
            })?
            .ok_or(InvoiceError::NotFound)?;

        let bytes = storage.download(&file.s3_key).await.map_err(|e| {
            tracing::error!("failed to download {} from S3: {e}", file.s3_key);
            InvoiceError::Internal
        })?;

        return Ok((bytes, inv.uuid));
    }

    tracing::warn!(invoice_id, uuid = %inv.uuid, extension, "no S3 file found, falling back to local disk");
    let path = std::path::Path::new(&inv.download_path).join(format!("{}.{}", inv.uuid, extension));
    let bytes = tokio::fs::read(&path).await.map_err(|e| {
        tracing::error!("failed to read invoice file {:?}: {e}", path);
        InvoiceError::NotFound
    })?;

    let s3_key = crate::storage::invoice_s3_key(user_id, &inv.uuid, extension);
    let pool = pool.clone();
    let inv_id = inv.id;
    let uuid = inv.uuid.clone();
    let extension = extension.to_string();
    let bytes_clone = bytes.clone();
    tokio::spawn(async move {
        match storage.upload(&s3_key, bytes_clone).await {
            Ok(()) => match files_repo::create(&pool, user_id, &s3_key, &extension).await {
                Ok(file) => {
                    if let Err(e) = invoice::set_file_id(&pool, inv_id, &extension, file.id).await {
                        tracing::error!(
                            invoice_id = inv_id,
                            "failed to set file_id after fallback upload: {e}"
                        );
                    } else {
                        tracing::info!(invoice_id = inv_id, uuid = %uuid, extension, "uploaded fallback file to S3");
                    }
                }
                Err(e) => tracing::error!(
                    invoice_id = inv_id,
                    "failed to save file record after fallback upload: {e}"
                ),
            },
            Err(e) => tracing::error!(
                invoice_id = inv_id,
                "failed to upload fallback file to S3: {e}"
            ),
        }
    });

    Ok((bytes, inv.uuid))
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    filters: InvoiceFilters,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Invoice>, i64), InvoiceError> {
    let (_, per_page, offset) = crate::services::paginate(page, per_page);
    invoice::list_for_user(pool, user_id, filters, per_page, offset)
        .await
        .map_err(|e| {
            tracing::error!("failed to list invoices: {e}");
            InvoiceError::Internal
        })
}
