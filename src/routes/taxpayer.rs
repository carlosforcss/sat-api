use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    extractors::AuthUser,
    repositories::taxpayer::{Taxpayer, TaxpayerFilters},
    services::taxpayer as taxpayer_service,
    AppState,
};

#[derive(Serialize, ToSchema)]
pub struct TaxpayerResponse {
    pub id: i32,
    pub taxpayer_id: String,
    pub name: String,
    pub cfdi_use: Option<String>,
    pub fiscal_domicile: Option<String>,
    pub fiscal_regime: Option<String>,
    pub foreign_tax_id: Option<String>,
    pub tax_residence: Option<String>,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct TaxpayerPage {
    pub data: Vec<TaxpayerResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl From<Taxpayer> for TaxpayerResponse {
    fn from(t: Taxpayer) -> Self {
        TaxpayerResponse {
            id: t.id,
            taxpayer_id: t.taxpayer_id,
            name: t.name,
            cfdi_use: t.cfdi_use,
            fiscal_domicile: t.fiscal_domicile,
            fiscal_regime: t.fiscal_regime,
            foreign_tax_id: t.foreign_tax_id,
            tax_residence: t.tax_residence,
            last_seen_at: t.last_seen_at,
            created_at: t.created_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct TaxpayerQueryParams {
    pub taxpayer_id: Option<String>,
    pub name: Option<String>,
    #[serde(default = "crate::routes::default_page")]
    pub page: i64,
    #[serde(default = "crate::routes::default_per_page")]
    pub per_page: i64,
}

#[utoipa::path(
    get,
    path = "/api/taxpayers",
    params(TaxpayerQueryParams),
    responses(
        (status = 200, description = "Paginated list of taxpayers", body = TaxpayerPage),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Taxpayers"
)]
pub async fn list_taxpayers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<TaxpayerQueryParams>,
) -> Response {
    let filters = TaxpayerFilters {
        taxpayer_id: params.taxpayer_id,
        name: params.name,
    };
    match taxpayer_service::list(
        &state.db,
        auth.user_id,
        filters,
        params.page,
        params.per_page,
    )
    .await
    {
        Ok((taxpayers, total)) => Json(TaxpayerPage {
            data: taxpayers.into_iter().map(TaxpayerResponse::from).collect(),
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
    path = "/api/taxpayers/{id}",
    params(("id" = i32, Path, description = "Taxpayer ID")),
    responses(
        (status = 200, description = "Taxpayer detail", body = TaxpayerResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Taxpayers"
)]
pub async fn get_taxpayer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Response {
    match taxpayer_service::get(&state.db, auth.user_id, id).await {
        Ok(t) => Json(TaxpayerResponse::from(t)).into_response(),
        Err(e) => e.into_response(),
    }
}
