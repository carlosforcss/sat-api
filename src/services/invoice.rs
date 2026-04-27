use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;

use chrono::{TimeZone, Utc};

use crate::repositories::files as files_repo;
use crate::repositories::invoice::{self, Invoice, InvoiceFilters};
use crate::repositories::invoice_item;
use crate::repositories::taxpayer::{self as taxpayer_repo, TaxpayerData};
use crate::storage::S3Storage;
use sat_cfdi;

pub enum InvoiceError {
    Internal,
    NotFound,
    ParseFailed(String),
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
            InvoiceError::ParseFailed(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": msg })),
            )
                .into_response(),
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

    Err(InvoiceError::NotFound)
}

async fn upsert_taxpayers(
    pool: &PgPool,
    user_id: i32,
    cfdi: &sat_cfdi::Invoice,
) -> (Option<i32>, Option<i32>) {
    let issued_at = sat_cfdi::parse_cfdi_datetime(&cfdi.issued_at)
        .map(|ndt| Utc.from_utc_datetime(&ndt))
        .unwrap_or_else(|_| Utc::now());

    let issuer_id = taxpayer_repo::upsert(
        pool,
        user_id,
        TaxpayerData {
            taxpayer_id: cfdi.issuer.taxpayer_id.clone(),
            name: cfdi
                .issuer
                .name
                .clone()
                .unwrap_or_else(|| cfdi.issuer.taxpayer_id.clone()),
            cfdi_use: None,
            fiscal_domicile: None,
            fiscal_regime: Some(cfdi.issuer.fiscal_regime.to_string()),
            foreign_tax_id: None,
            tax_residence: None,
            last_seen_at: issued_at,
        },
    )
    .await
    .map_err(|e| tracing::error!("failed to upsert issuer taxpayer: {e}"))
    .ok();

    let receiver_id = taxpayer_repo::upsert(
        pool,
        user_id,
        TaxpayerData {
            taxpayer_id: cfdi.recipient.taxpayer_id.clone(),
            name: cfdi
                .recipient
                .name
                .clone()
                .unwrap_or_else(|| cfdi.recipient.taxpayer_id.clone()),
            cfdi_use: Some(cfdi.recipient.cfdi_use.to_string()),
            fiscal_domicile: cfdi.recipient.fiscal_domicile.clone(),
            fiscal_regime: cfdi.recipient.fiscal_regime.as_ref().map(|v| v.to_string()),
            foreign_tax_id: cfdi.recipient.foreign_tax_id.clone(),
            tax_residence: cfdi.recipient.tax_residence.clone(),
            last_seen_at: issued_at,
        },
    )
    .await
    .map_err(|e| tracing::error!("failed to upsert receiver taxpayer: {e}"))
    .ok();

    (issuer_id, receiver_id)
}

fn cfdi_to_parsed_data(
    cfdi: &sat_cfdi::Invoice,
    issuer_id: Option<i32>,
    receiver_id: Option<i32>,
) -> invoice::ParsedData {
    invoice::ParsedData {
        issuer_id,
        receiver_id,
        version: cfdi.version.clone(),
        series: cfdi.series.clone(),
        payment_form: cfdi.payment_form.as_ref().map(|v| v.to_string()),
        payment_conditions: cfdi.payment_conditions.clone(),
        subtotal: cfdi.subtotal.parse().ok(),
        discount: cfdi.discount.as_deref().and_then(|d| d.parse().ok()),
        currency: cfdi.currency.to_string(),
        exchange_rate: cfdi.exchange_rate.as_deref().and_then(|r| r.parse().ok()),
        export: cfdi.export.as_ref().map(|v| v.to_string()),
        payment_method: cfdi.payment_method.as_ref().map(|v| v.to_string()),
        issue_place: cfdi.issue_place.clone(),
        certificate_number: cfdi.certificate_number.clone(),
        cfdi_use: cfdi.recipient.cfdi_use.to_string(),
        issuer_fiscal_regime: cfdi.issuer.fiscal_regime.to_string(),
        recipient_fiscal_regime: cfdi.recipient.fiscal_regime.as_ref().map(|v| v.to_string()),
    }
}

pub async fn parse_invoice(
    pool: &PgPool,
    storage: Arc<S3Storage>,
    user_id: i32,
    invoice_id: i32,
) -> Result<serde_json::Value, InvoiceError> {
    let (bytes, _uuid) = get_invoice_file(pool, storage, user_id, invoice_id, "xml").await?;

    match sat_cfdi::parse_bytes(&bytes) {
        Ok(cfdi) => {
            let (issuer_id, receiver_id) = upsert_taxpayers(pool, user_id, &cfdi).await;
            invoice::set_parse_result(
                pool,
                invoice_id,
                cfdi_to_parsed_data(&cfdi, issuer_id, receiver_id),
            )
            .await
            .map_err(|e| {
                tracing::error!("failed to persist parse result for invoice {invoice_id}: {e}");
                InvoiceError::Internal
            })?;
            if let Err(e) =
                invoice_item::replace_for_invoice(pool, invoice_id, cfdi.line_items()).await
            {
                tracing::error!("failed to persist items for invoice {invoice_id}: {e}");
                return Err(InvoiceError::Internal);
            }
            Ok(serde_json::to_value(cfdi).unwrap())
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = invoice::set_parse_error(pool, invoice_id, &msg).await;
            Err(InvoiceError::ParseFailed(msg))
        }
    }
}

pub struct ParseAllResult {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
}

pub async fn parse_all(
    pool: &PgPool,
    storage: Arc<S3Storage>,
    user_id: i32,
    force: bool,
) -> Result<ParseAllResult, InvoiceError> {
    let invoices = invoice::list_with_xml_for_user(pool, user_id, force)
        .await
        .map_err(|e| {
            tracing::error!("failed to list invoices for bulk parse: {e}");
            InvoiceError::Internal
        })?;

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for inv in &invoices {
        let file_id = inv.xml_file_id.unwrap();
        let s3_key = match files_repo::find_by_id(pool, file_id).await {
            Ok(Some(f)) => f.s3_key,
            Ok(None) => {
                tracing::error!(
                    "bulk parse: file record {file_id} not found for invoice {}",
                    inv.id
                );
                failed += 1;
                continue;
            }
            Err(e) => {
                tracing::error!("bulk parse: db error fetching file {file_id}: {e}");
                failed += 1;
                continue;
            }
        };
        let bytes = match storage.download(&s3_key).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(
                    "bulk parse: failed to download xml for invoice {}: {e}",
                    inv.id
                );
                failed += 1;
                continue;
            }
        };
        match sat_cfdi::parse_bytes(&bytes) {
            Ok(cfdi) => {
                let (issuer_id, receiver_id) = upsert_taxpayers(pool, user_id, &cfdi).await;
                let parse_ok = invoice::set_parse_result(
                    pool,
                    inv.id,
                    cfdi_to_parsed_data(&cfdi, issuer_id, receiver_id),
                )
                .await
                .is_ok();
                let items_ok = if parse_ok {
                    invoice_item::replace_for_invoice(pool, inv.id, cfdi.line_items())
                        .await
                        .map_err(|e| {
                            tracing::error!(
                                "bulk parse: failed to persist items for invoice {}: {e}",
                                inv.id
                            )
                        })
                        .is_ok()
                } else {
                    false
                };
                if parse_ok && items_ok {
                    succeeded += 1;
                } else {
                    failed += 1;
                }
            }
            Err(e) => {
                let _ = invoice::set_parse_error(pool, inv.id, &e.to_string()).await;
                failed += 1;
            }
        }
    }

    Ok(ParseAllResult {
        processed: invoices.len(),
        succeeded,
        failed,
    })
}

pub async fn get(pool: &PgPool, user_id: i32, invoice_id: i32) -> Result<Invoice, InvoiceError> {
    invoice::find_by_id_for_user(pool, invoice_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch invoice {invoice_id}: {e}");
            InvoiceError::Internal
        })?
        .ok_or(InvoiceError::NotFound)
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
