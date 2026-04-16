use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    extractors::AuthUser,
    repositories::invoice::{Invoice, InvoiceFilters},
    services::invoice as invoice_service,
    AppState,
};

impl From<Invoice> for InvoiceResponse {
    fn from(inv: Invoice) -> Self {
        InvoiceResponse {
            id: inv.id,
            link_id: inv.link_id,
            uuid: inv.uuid,
            fiscal_id: inv.fiscal_id,
            issuer_taxpayer_id: inv.issuer_taxpayer_id,
            issuer_name: inv.issuer_name,
            receiver_taxpayer_id: inv.receiver_taxpayer_id,
            receiver_name: inv.receiver_name,
            issued_at: inv.issued_at,
            certified_at: inv.certified_at,
            total: inv.total,
            invoice_type: inv.invoice_type,
            invoice_status: inv.invoice_status,
            has_xml: inv.xml_file_id.is_some(),
            has_pdf: inv.pdf_file_id.is_some(),
            created_at: inv.created_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct InvoiceQueryParams {
    pub issuer_taxpayer_id: Option<String>,
    pub receiver_taxpayer_id: Option<String>,
    pub invoice_type: Option<String>,
    pub invoice_status: Option<String>,
    pub has_xml: Option<bool>,
    pub has_pdf: Option<bool>,
    #[serde(default = "crate::routes::default_page")]
    pub page: i64,
    #[serde(default = "crate::routes::default_per_page")]
    pub per_page: i64,
}

#[derive(Serialize, ToSchema)]
pub struct InvoicePage {
    pub data: Vec<InvoiceResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Serialize, ToSchema)]
pub struct InvoiceResponse {
    pub id: i32,
    pub link_id: i32,
    pub uuid: String,
    pub fiscal_id: String,
    pub issuer_taxpayer_id: String,
    pub issuer_name: String,
    pub receiver_taxpayer_id: String,
    pub receiver_name: String,
    pub issued_at: String,
    pub certified_at: String,
    pub total: String,
    pub invoice_type: String,
    pub invoice_status: String,
    pub has_xml: bool,
    pub has_pdf: bool,
    pub created_at: DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/api/invoices",
    params(InvoiceQueryParams),
    responses(
        (status = 200, description = "Paginated list of invoices", body = InvoicePage),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Invoices"
)]
pub async fn list_invoices(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<InvoiceQueryParams>,
) -> Response {
    let filters = InvoiceFilters {
        issuer_taxpayer_id: params.issuer_taxpayer_id,
        receiver_taxpayer_id: params.receiver_taxpayer_id,
        invoice_type: params.invoice_type,
        invoice_status: params.invoice_status,
        has_xml: params.has_xml,
        has_pdf: params.has_pdf,
    };

    match invoice_service::list(
        &state.db,
        auth.user_id,
        filters,
        params.page,
        params.per_page,
    )
    .await
    {
        Ok((invoices, total)) => Json(InvoicePage {
            data: invoices.into_iter().map(InvoiceResponse::from).collect(),
            total,
            page: params.page,
            per_page: params.per_page,
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/invoices/{invoice_id}/xml",
    params(("invoice_id" = i32, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice XML file", content_type = "application/xml"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Invoices"
)]
pub async fn get_invoice_xml(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(invoice_id): Path<i32>,
) -> Response {
    serve_invoice_file(&state, auth.user_id, invoice_id, "xml", "application/xml").await
}

#[utoipa::path(
    get,
    path = "/api/invoices/{invoice_id}/pdf",
    params(("invoice_id" = i32, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice PDF file", content_type = "application/pdf"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Invoices"
)]
pub async fn get_invoice_pdf(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(invoice_id): Path<i32>,
) -> Response {
    serve_invoice_file(&state, auth.user_id, invoice_id, "pdf", "application/pdf").await
}

async fn serve_invoice_file(
    state: &AppState,
    user_id: i32,
    invoice_id: i32,
    extension: &str,
    content_type: &str,
) -> Response {
    match invoice_service::get_invoice_file(
        &state.db,
        Arc::clone(&state.storage),
        user_id,
        invoice_id,
        extension,
    )
    .await
    {
        Ok((bytes, uuid)) => Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{uuid}.{extension}\""),
            )
            .body(Body::from(bytes))
            .unwrap(),
        Err(e) => e.into_response(),
    }
}
