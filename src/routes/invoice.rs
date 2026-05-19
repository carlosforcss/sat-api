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
    repositories::invoice_item::{InvoiceItem, InvoiceItemTax},
    repositories::invoice_payment::{
        InvoicePayment, PaymentComplement, PaymentDocumentTax, PaymentRelatedDocument,
    },
    repositories::invoice_related_document::RelatedDocument,
    services::invoice::{self as invoice_service, InvoiceError},
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
            issuer_id: inv.issuer_id,
            receiver_id: inv.receiver_id,
            parsed: inv.parsed,
            parsing_error: inv.parsing_error,
            version: inv.version,
            series: inv.series,
            payment_form: inv.payment_form,
            payment_conditions: inv.payment_conditions,
            subtotal: inv.subtotal,
            discount: inv.discount,
            currency: inv.currency,
            exchange_rate: inv.exchange_rate,
            export: inv.export,
            payment_method: inv.payment_method,
            issue_place: inv.issue_place,
            certificate_number: inv.certificate_number,
            cfdi_use: inv.cfdi_use,
            issuer_fiscal_regime: inv.issuer_fiscal_regime,
            recipient_fiscal_regime: inv.recipient_fiscal_regime,
            created_at: inv.created_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct InvoiceQueryParams {
    // existing
    pub issuer_taxpayer_id: Option<String>,
    pub receiver_taxpayer_id: Option<String>,
    pub invoice_type: Option<String>,
    pub invoice_status: Option<String>,
    pub has_xml: Option<bool>,
    pub has_pdf: Option<bool>,
    // identity
    pub uuid: Option<String>,
    pub fiscal_id: Option<String>,
    pub issuer_name: Option<String>,
    pub receiver_name: Option<String>,
    // fiscal scalars
    pub version: Option<String>,
    pub series: Option<String>,
    pub payment_form: Option<String>,
    pub currency: Option<String>,
    pub export: Option<String>,
    pub payment_method: Option<String>,
    pub issue_place: Option<String>,
    pub cfdi_use: Option<String>,
    pub issuer_fiscal_regime: Option<String>,
    pub recipient_fiscal_regime: Option<String>,
    // parse state
    pub parsed: Option<bool>,
    // taxpayer FK
    pub issuer_id: Option<i32>,
    pub receiver_id: Option<i32>,
    pub taxpayer_id: Option<i32>,
    // ranges
    pub issued_from: Option<DateTime<Utc>>,
    pub issued_to: Option<DateTime<Utc>>,
    pub total_min: Option<f64>,
    pub total_max: Option<f64>,
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
    pub link_id: Option<i32>,
    pub uuid: String,
    pub fiscal_id: String,
    pub issuer_taxpayer_id: String,
    pub issuer_name: String,
    pub receiver_taxpayer_id: String,
    pub receiver_name: String,
    pub issued_at: DateTime<Utc>,
    pub certified_at: DateTime<Utc>,
    pub total: f64,
    pub invoice_type: String,
    pub invoice_status: String,
    pub has_xml: bool,
    pub has_pdf: bool,
    pub issuer_id: Option<i32>,
    pub receiver_id: Option<i32>,
    pub parsed: Option<bool>,
    pub parsing_error: Option<String>,
    pub version: Option<String>,
    pub series: Option<String>,
    pub payment_form: Option<String>,
    pub payment_conditions: Option<String>,
    pub subtotal: Option<f64>,
    pub discount: Option<f64>,
    pub currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub export: Option<String>,
    pub payment_method: Option<String>,
    pub issue_place: Option<String>,
    pub certificate_number: Option<String>,
    pub cfdi_use: Option<String>,
    pub issuer_fiscal_regime: Option<String>,
    pub recipient_fiscal_regime: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct RelatedDocumentResponse {
    pub id: i32,
    pub relation_type: String,
    pub related_uuid: String,
    pub related_invoice_id: Option<i32>,
}

impl From<RelatedDocument> for RelatedDocumentResponse {
    fn from(r: RelatedDocument) -> Self {
        RelatedDocumentResponse {
            id: r.id,
            relation_type: r.relation_type,
            related_uuid: r.related_uuid,
            related_invoice_id: r.related_invoice_id,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentDocumentTaxResponse {
    pub id: i32,
    pub tax_type: String,
    pub tax: String,
    pub factor_type: Option<String>,
    pub base: Option<f64>,
    pub rate_or_amount: Option<f64>,
    pub amount: Option<f64>,
}

impl From<PaymentDocumentTax> for PaymentDocumentTaxResponse {
    fn from(t: PaymentDocumentTax) -> Self {
        PaymentDocumentTaxResponse {
            id: t.id,
            tax_type: t.tax_type,
            tax: t.tax,
            factor_type: t.factor_type,
            base: t.base,
            rate_or_amount: t.rate_or_amount,
            amount: t.amount,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentRelatedDocumentResponse {
    pub id: i32,
    pub document_id: String,
    pub related_invoice_id: Option<i32>,
    pub series: Option<String>,
    pub fiscal_id: Option<String>,
    pub document_currency: String,
    pub exchange_equivalence: Option<f64>,
    pub installment_number: i32,
    pub previous_balance: f64,
    pub paid_amount: f64,
    pub outstanding_balance: f64,
    pub tax_object: String,
    pub total_transferred_tax: f64,
    pub total_withheld_tax: f64,
    pub taxes: Vec<PaymentDocumentTaxResponse>,
}

impl From<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)> for PaymentRelatedDocumentResponse {
    fn from((d, taxes): (PaymentRelatedDocument, Vec<PaymentDocumentTax>)) -> Self {
        PaymentRelatedDocumentResponse {
            id: d.id,
            document_id: d.document_id,
            related_invoice_id: d.related_invoice_id,
            series: d.series,
            fiscal_id: d.fiscal_id,
            document_currency: d.document_currency,
            exchange_equivalence: d.exchange_equivalence,
            installment_number: d.installment_number,
            previous_balance: d.previous_balance,
            paid_amount: d.paid_amount,
            outstanding_balance: d.outstanding_balance,
            tax_object: d.tax_object,
            total_transferred_tax: d.total_transferred_tax,
            total_withheld_tax: d.total_withheld_tax,
            taxes: taxes.into_iter().map(PaymentDocumentTaxResponse::from).collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentResponse {
    pub id: i32,
    pub complement_id: i32,
    pub invoice_id: i32,
    pub payment_date: DateTime<Utc>,
    pub payment_form: String,
    pub currency: String,
    pub exchange_rate: Option<f64>,
    pub amount: f64,
    pub operation_number: Option<String>,
    pub ordering_account_issuer_tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub ordering_account: Option<String>,
    pub beneficiary_account_issuer_tax_id: Option<String>,
    pub beneficiary_account: Option<String>,
    pub total_transferred_tax: f64,
    pub total_withheld_tax: f64,
    pub related_documents: Vec<PaymentRelatedDocumentResponse>,
}

impl From<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>)>
    for PaymentResponse
{
    fn from(
        (p, docs): (InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>),
    ) -> Self {
        PaymentResponse {
            id: p.id,
            complement_id: p.complement_id,
            invoice_id: p.invoice_id,
            payment_date: p.payment_date,
            payment_form: p.payment_form,
            currency: p.currency,
            exchange_rate: p.exchange_rate,
            amount: p.amount,
            operation_number: p.operation_number,
            ordering_account_issuer_tax_id: p.ordering_account_issuer_tax_id,
            bank_name: p.bank_name,
            ordering_account: p.ordering_account,
            beneficiary_account_issuer_tax_id: p.beneficiary_account_issuer_tax_id,
            beneficiary_account: p.beneficiary_account,
            total_transferred_tax: p.total_transferred_tax,
            total_withheld_tax: p.total_withheld_tax,
            related_documents: docs
                .into_iter()
                .map(PaymentRelatedDocumentResponse::from)
                .collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentComplementResponse {
    pub id: i32,
    pub invoice_id: i32,
    pub version: String,
    pub total_payments_amount: Option<f64>,
    pub total_iva_withheld: Option<f64>,
    pub total_isr_withheld: Option<f64>,
    pub total_ieps_withheld: Option<f64>,
    pub total_transferred_iva_base_16: Option<f64>,
    pub total_transferred_iva_tax_16: Option<f64>,
    pub total_transferred_iva_base_8: Option<f64>,
    pub total_transferred_iva_tax_8: Option<f64>,
    pub total_transferred_iva_base_0: Option<f64>,
    pub total_transferred_iva_tax_0: Option<f64>,
    pub total_transferred_iva_base_exempt: Option<f64>,
    pub payments: Vec<PaymentResponse>,
}

impl
    From<(
        PaymentComplement,
        Vec<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>)>,
    )> for PaymentComplementResponse
{
    fn from(
        (c, payments): (
            PaymentComplement,
            Vec<(InvoicePayment, Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>)>,
        ),
    ) -> Self {
        PaymentComplementResponse {
            id: c.id,
            invoice_id: c.invoice_id,
            version: c.version,
            total_payments_amount: c.total_payments_amount,
            total_iva_withheld: c.total_iva_withheld,
            total_isr_withheld: c.total_isr_withheld,
            total_ieps_withheld: c.total_ieps_withheld,
            total_transferred_iva_base_16: c.total_transferred_iva_base_16,
            total_transferred_iva_tax_16: c.total_transferred_iva_tax_16,
            total_transferred_iva_base_8: c.total_transferred_iva_base_8,
            total_transferred_iva_tax_8: c.total_transferred_iva_tax_8,
            total_transferred_iva_base_0: c.total_transferred_iva_base_0,
            total_transferred_iva_tax_0: c.total_transferred_iva_tax_0,
            total_transferred_iva_base_exempt: c.total_transferred_iva_base_exempt,
            payments: payments.into_iter().map(PaymentResponse::from).collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentListResponse {
    pub id: i32,
    pub complement_id: i32,
    pub invoice_id: i32,
    pub payment_date: DateTime<Utc>,
    pub payment_form: String,
    pub currency: String,
    pub exchange_rate: Option<f64>,
    pub amount: f64,
    pub operation_number: Option<String>,
    pub ordering_account_issuer_tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub ordering_account: Option<String>,
    pub beneficiary_account_issuer_tax_id: Option<String>,
    pub beneficiary_account: Option<String>,
    pub total_transferred_tax: f64,
    pub total_withheld_tax: f64,
}

impl From<InvoicePayment> for PaymentListResponse {
    fn from(p: InvoicePayment) -> Self {
        PaymentListResponse {
            id: p.id,
            complement_id: p.complement_id,
            invoice_id: p.invoice_id,
            payment_date: p.payment_date,
            payment_form: p.payment_form,
            currency: p.currency,
            exchange_rate: p.exchange_rate,
            amount: p.amount,
            operation_number: p.operation_number,
            ordering_account_issuer_tax_id: p.ordering_account_issuer_tax_id,
            bank_name: p.bank_name,
            ordering_account: p.ordering_account,
            beneficiary_account_issuer_tax_id: p.beneficiary_account_issuer_tax_id,
            beneficiary_account: p.beneficiary_account,
            total_transferred_tax: p.total_transferred_tax,
            total_withheld_tax: p.total_withheld_tax,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentPage {
    pub data: Vec<PaymentListResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Serialize, ToSchema)]
pub struct InvoiceItemTaxResponse {
    pub id: i32,
    pub tax_type: String,
    pub tax: String,
    pub factor_type: Option<String>,
    pub base: Option<f64>,
    pub rate_or_amount: Option<f64>,
    pub amount: Option<f64>,
}

impl From<InvoiceItemTax> for InvoiceItemTaxResponse {
    fn from(t: InvoiceItemTax) -> Self {
        InvoiceItemTaxResponse {
            id: t.id,
            tax_type: t.tax_type,
            tax: t.tax,
            factor_type: t.factor_type,
            base: t.base,
            rate_or_amount: t.rate_or_amount,
            amount: t.amount,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct InvoiceItemResponse {
    pub id: i32,
    pub product_service_key: String,
    pub id_number: Option<String>,
    pub quantity: f64,
    pub unit_key: String,
    pub unit: Option<String>,
    pub description: String,
    pub unit_value: f64,
    pub amount: f64,
    pub discount: Option<f64>,
    pub tax_object: Option<String>,
    pub taxes: Vec<InvoiceItemTaxResponse>,
    pub third_party: Option<serde_json::Value>,
    pub customs_info: serde_json::Value,
    pub property_tax_accounts: serde_json::Value,
    pub parts: serde_json::Value,
}

impl From<(InvoiceItem, Vec<InvoiceItemTax>)> for InvoiceItemResponse {
    fn from((item, taxes): (InvoiceItem, Vec<InvoiceItemTax>)) -> Self {
        InvoiceItemResponse {
            id: item.id,
            product_service_key: item.product_service_key,
            id_number: item.id_number,
            quantity: item.quantity,
            unit_key: item.unit_key,
            unit: item.unit,
            description: item.description,
            unit_value: item.unit_value,
            amount: item.amount,
            discount: item.discount,
            tax_object: item.tax_object,
            taxes: taxes
                .into_iter()
                .map(InvoiceItemTaxResponse::from)
                .collect(),
            third_party: item.third_party,
            customs_info: item.customs_info,
            property_tax_accounts: item.property_tax_accounts,
            parts: item.parts,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct InvoiceDetailResponse {
    pub id: i32,
    pub link_id: Option<i32>,
    pub uuid: String,
    pub fiscal_id: String,
    pub issuer_taxpayer_id: String,
    pub issuer_name: String,
    pub receiver_taxpayer_id: String,
    pub receiver_name: String,
    pub issued_at: DateTime<Utc>,
    pub certified_at: DateTime<Utc>,
    pub total: f64,
    pub invoice_type: String,
    pub invoice_status: String,
    pub has_xml: bool,
    pub has_pdf: bool,
    pub issuer_id: Option<i32>,
    pub receiver_id: Option<i32>,
    pub parsed: Option<bool>,
    pub parsing_error: Option<String>,
    pub version: Option<String>,
    pub series: Option<String>,
    pub payment_form: Option<String>,
    pub payment_conditions: Option<String>,
    pub subtotal: Option<f64>,
    pub discount: Option<f64>,
    pub currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub export: Option<String>,
    pub payment_method: Option<String>,
    pub issue_place: Option<String>,
    pub certificate_number: Option<String>,
    pub cfdi_use: Option<String>,
    pub issuer_fiscal_regime: Option<String>,
    pub recipient_fiscal_regime: Option<String>,
    pub created_at: DateTime<Utc>,
    pub items: Vec<InvoiceItemResponse>,
    pub related_documents: Vec<RelatedDocumentResponse>,
    pub payment_complement: Option<PaymentComplementResponse>,
}

impl InvoiceDetailResponse {
    pub fn from_parts(
        inv: Invoice,
        items: Vec<(InvoiceItem, Vec<InvoiceItemTax>)>,
        related_documents: Vec<RelatedDocument>,
        payment_complement: Option<(
            PaymentComplement,
            Vec<(
                InvoicePayment,
                Vec<(PaymentRelatedDocument, Vec<PaymentDocumentTax>)>,
            )>,
        )>,
    ) -> Self {
        InvoiceDetailResponse {
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
            issuer_id: inv.issuer_id,
            receiver_id: inv.receiver_id,
            parsed: inv.parsed,
            parsing_error: inv.parsing_error,
            version: inv.version,
            series: inv.series,
            payment_form: inv.payment_form,
            payment_conditions: inv.payment_conditions,
            subtotal: inv.subtotal,
            discount: inv.discount,
            currency: inv.currency,
            exchange_rate: inv.exchange_rate,
            export: inv.export,
            payment_method: inv.payment_method,
            issue_place: inv.issue_place,
            certificate_number: inv.certificate_number,
            cfdi_use: inv.cfdi_use,
            issuer_fiscal_regime: inv.issuer_fiscal_regime,
            recipient_fiscal_regime: inv.recipient_fiscal_regime,
            created_at: inv.created_at,
            items: items.into_iter().map(InvoiceItemResponse::from).collect(),
            related_documents: related_documents
                .into_iter()
                .map(RelatedDocumentResponse::from)
                .collect(),
            payment_complement: payment_complement.map(PaymentComplementResponse::from),
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct ParseAllParams {
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ParseAllResponse {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
}

#[utoipa::path(
    post,
    path = "/api/invoices/parse-all",
    params(ParseAllParams),
    responses(
        (status = 200, description = "Bulk parse complete", body = ParseAllResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal error"),
    ),
    security(("bearer_auth" = [])),
    tag = "Invoices"
)]
pub async fn parse_all_invoices(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ParseAllParams>,
) -> Response {
    match invoice_service::parse_all(
        &state.db,
        Arc::clone(&state.storage),
        auth.user_id,
        params.force,
    )
    .await
    {
        Ok(result) => Json(ParseAllResponse {
            processed: result.processed,
            succeeded: result.succeeded,
            failed: result.failed,
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
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
        uuid: params.uuid,
        fiscal_id: params.fiscal_id,
        issuer_name: params.issuer_name,
        receiver_name: params.receiver_name,
        version: params.version,
        series: params.series,
        payment_form: params.payment_form,
        currency: params.currency,
        export: params.export,
        payment_method: params.payment_method,
        issue_place: params.issue_place,
        cfdi_use: params.cfdi_use,
        issuer_fiscal_regime: params.issuer_fiscal_regime,
        recipient_fiscal_regime: params.recipient_fiscal_regime,
        parsed: params.parsed,
        issuer_id: params.issuer_id,
        receiver_id: params.receiver_id,
        issued_from: params.issued_from,
        issued_to: params.issued_to,
        total_min: params.total_min,
        total_max: params.total_max,
        taxpayer_id: params.taxpayer_id,
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
    path = "/api/invoices/{invoice_id}",
    params(("invoice_id" = i32, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice detail with items", body = InvoiceDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Invoices"
)]
pub async fn get_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(invoice_id): Path<i32>,
) -> Response {
    let inv = match invoice_service::get(&state.db, auth.user_id, invoice_id).await {
        Ok(inv) => inv,
        Err(e) => return e.into_response(),
    };

    let items =
        match crate::repositories::invoice_item::list_for_invoice(&state.db, invoice_id, auth.user_id)
            .await
        {
            Ok(items) => items,
            Err(e) => {
                tracing::error!("failed to fetch items for invoice {invoice_id}: {e}");
                return InvoiceError::Internal.into_response();
            }
        };

    let related_documents = match crate::repositories::invoice_related_document::list_for_invoice(
        &state.db,
        invoice_id,
        auth.user_id,
    )
    .await
    {
        Ok(docs) => docs,
        Err(e) => {
            tracing::error!("failed to fetch related documents for invoice {invoice_id}: {e}");
            return InvoiceError::Internal.into_response();
        }
    };

    let payment_complement = match crate::repositories::invoice_payment::find_for_invoice(
        &state.db,
        invoice_id,
        auth.user_id,
    )
    .await
    {
        Ok(pc) => pc,
        Err(e) => {
            tracing::error!("failed to fetch payment complement for invoice {invoice_id}: {e}");
            return InvoiceError::Internal.into_response();
        }
    };

    Json(InvoiceDetailResponse::from_parts(inv, items, related_documents, payment_complement))
        .into_response()
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

#[utoipa::path(
    get,
    path = "/api/invoices/{invoice_id}/parse",
    params(("invoice_id" = i32, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Parsed CFDI invoice as JSON"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found or no XML attached"),
        (status = 422, description = "XML could not be parsed as CFDI"),
    ),
    security(("bearer_auth" = [])),
    tag = "Invoices"
)]
pub async fn parse_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(invoice_id): Path<i32>,
) -> Response {
    match invoice_service::parse_invoice(
        &state.db,
        Arc::clone(&state.storage),
        auth.user_id,
        invoice_id,
    )
    .await
    {
        Ok(value) => Json(value).into_response(),
        Err(e) => e.into_response(),
    }
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
