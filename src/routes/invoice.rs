use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    extractors::AuthUser,
    repositories::invoice::InvoiceFilters,
    services::invoice as invoice_service,
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct InvoiceQueryParams {
    pub issuer_taxpayer_id: Option<String>,
    pub receiver_taxpayer_id: Option<String>,
    pub invoice_type: Option<String>,
    pub invoice_status: Option<String>,
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
    pub download_path: String,
    pub created_at: DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/api/invoices",
    params(InvoiceQueryParams),
    responses(
        (status = 200, description = "List of invoices", body = Vec<InvoiceResponse>),
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
    };

    match invoice_service::list(&state.db, auth.user_id, filters).await {
        Ok(invoices) => Json(
            invoices
                .into_iter()
                .map(|inv| InvoiceResponse {
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
                    download_path: inv.download_path,
                    created_at: inv.created_at,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => e.into_response(),
    }
}
