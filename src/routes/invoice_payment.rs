use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    extractors::AuthUser,
    repositories::invoice_payment::PaymentFilters,
    routes::invoice::{PaymentListResponse, PaymentPage, PaymentResponse},
    AppState,
};

#[derive(Deserialize, IntoParams)]
pub struct PaymentQueryParams {
    pub invoice_id: Option<i32>,
    pub payment_form: Option<String>,
    pub currency: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub amount_min: Option<f64>,
    pub amount_max: Option<f64>,
    #[serde(default = "crate::routes::default_page")]
    pub page: i64,
    #[serde(default = "crate::routes::default_per_page")]
    pub per_page: i64,
}

#[utoipa::path(
    get,
    path = "/api/invoices/payments",
    params(PaymentQueryParams),
    responses(
        (status = 200, description = "Paginated list of payments", body = PaymentPage),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Payments"
)]
pub async fn list_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaymentQueryParams>,
) -> Response {
    let (page, per_page, offset) = crate::services::paginate(params.page, params.per_page);

    let filters = PaymentFilters {
        invoice_id: params.invoice_id,
        payment_form: params.payment_form,
        currency: params.currency,
        date_from: params.date_from,
        date_to: params.date_to,
        amount_min: params.amount_min,
        amount_max: params.amount_max,
    };

    match crate::services::invoice_payment::list(&state.db, auth.user_id, filters, per_page, offset)
        .await
    {
        Ok((payments, total)) => Json(PaymentPage {
            data: payments.into_iter().map(PaymentListResponse::from).collect(),
            total,
            page,
            per_page,
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/invoices/payments/{payment_id}",
    params(("payment_id" = i32, Path, description = "Payment ID")),
    responses(
        (status = 200, description = "Payment detail with related documents", body = PaymentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Payments"
)]
pub async fn get_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(payment_id): Path<i32>,
) -> Response {
    match crate::services::invoice_payment::get(&state.db, auth.user_id, payment_id).await {
        Ok(row) => Json(PaymentResponse::from(row)).into_response(),
        Err(e) => e.into_response(),
    }
}
