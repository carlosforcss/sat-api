use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use utoipa::IntoParams;

#[derive(FromRow, Clone)]
pub struct Invoice {
    pub id: i32,
    pub user_id: i32,
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
    pub xml_file_id: Option<i32>,
    pub pdf_file_id: Option<i32>,
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

#[derive(Deserialize, IntoParams)]
pub struct InvoiceFilters {
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
    // either side taxpayer FK
    pub taxpayer_id: Option<i32>,
    // ranges
    pub issued_from: Option<DateTime<Utc>>,
    pub issued_to: Option<DateTime<Utc>>,
    pub total_min: Option<f64>,
    pub total_max: Option<f64>,
    // pagination (used by routes, ignored by the repo query)
    #[serde(default = "crate::routes::default_page")]
    pub page: i64,
    #[serde(default = "crate::routes::default_per_page")]
    pub per_page: i64,
}

pub struct ParsedData {
    pub issuer_id: Option<i32>,
    pub receiver_id: Option<i32>,
    pub invoice_type: String,
    pub version: String,
    pub series: Option<String>,
    pub payment_form: Option<String>,
    pub payment_conditions: Option<String>,
    pub subtotal: Option<f64>,
    pub discount: Option<f64>,
    pub currency: String,
    pub exchange_rate: Option<f64>,
    pub export: Option<String>,
    pub payment_method: Option<String>,
    pub issue_place: String,
    pub certificate_number: String,
    pub cfdi_use: String,
    pub issuer_fiscal_regime: String,
    pub recipient_fiscal_regime: Option<String>,
}

const SELECT_COLUMNS: &str = "
    id, user_id, link_id, uuid::TEXT, fiscal_id, issuer_taxpayer_id, issuer_name,
    receiver_taxpayer_id, receiver_name, issued_at, certified_at, total::FLOAT8,
    invoice_type::TEXT, invoice_status::TEXT, xml_file_id, pdf_file_id,
    issuer_id, receiver_id,
    parsed, parsing_error, version, series, payment_form, payment_conditions,
    subtotal::FLOAT8, discount::FLOAT8, currency, exchange_rate::FLOAT8,
    export, payment_method, issue_place, certificate_number,
    cfdi_use, issuer_fiscal_regime, recipient_fiscal_regime, created_at";

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    link_id: Option<i32>,
    uuid: &str,
    fiscal_id: &str,
    issuer_taxpayer_id: &str,
    issuer_name: &str,
    receiver_taxpayer_id: &str,
    receiver_name: &str,
    issued_at: DateTime<Utc>,
    certified_at: DateTime<Utc>,
    total: f64,
    invoice_type: &str,
    invoice_status: &str,
) -> Result<Invoice, sqlx::Error> {
    sqlx::query_as::<_, Invoice>(
        &format!(
            "INSERT INTO invoices (user_id, link_id, uuid, fiscal_id, issuer_taxpayer_id, issuer_name,
                                   receiver_taxpayer_id, receiver_name, issued_at, certified_at,
                                   total, invoice_type, invoice_status)
             VALUES ($1, $2, $3::UUID, $4, $5, $6, $7, $8, $9, $10, $11, $12::invoice_type_enum, $13::invoice_status_enum)
             ON CONFLICT (uuid, user_id) DO UPDATE SET
                 link_id              = EXCLUDED.link_id,
                 fiscal_id            = EXCLUDED.fiscal_id,
                 issuer_taxpayer_id   = EXCLUDED.issuer_taxpayer_id,
                 issuer_name          = EXCLUDED.issuer_name,
                 receiver_taxpayer_id = EXCLUDED.receiver_taxpayer_id,
                 receiver_name        = EXCLUDED.receiver_name,
                 issued_at            = EXCLUDED.issued_at,
                 certified_at         = EXCLUDED.certified_at,
                 total                = EXCLUDED.total,
                 invoice_type         = EXCLUDED.invoice_type,
                 invoice_status       = EXCLUDED.invoice_status
             RETURNING {SELECT_COLUMNS}"
        ),
    )
    .bind(user_id)
    .bind(link_id)
    .bind(uuid)
    .bind(fiscal_id)
    .bind(issuer_taxpayer_id)
    .bind(issuer_name)
    .bind(receiver_taxpayer_id)
    .bind(receiver_name)
    .bind(issued_at)
    .bind(certified_at)
    .bind(total)
    .bind(invoice_type)
    .bind(invoice_status)
    .fetch_one(pool)
    .await
}

pub async fn find_by_uuid_and_user(
    pool: &PgPool,
    uuid: &str,
    user_id: i32,
) -> Result<Option<Invoice>, sqlx::Error> {
    sqlx::query_as::<_, Invoice>(&format!(
        "SELECT {SELECT_COLUMNS} FROM invoices WHERE uuid = $1::UUID AND user_id = $2"
    ))
    .bind(uuid)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id_for_user(
    pool: &PgPool,
    id: i32,
    user_id: i32,
) -> Result<Option<Invoice>, sqlx::Error> {
    sqlx::query_as::<_, Invoice>(&format!(
        "SELECT {SELECT_COLUMNS} FROM invoices WHERE id = $1 AND user_id = $2"
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_for_user(
    pool: &PgPool,
    user_id: i32,
    filters: InvoiceFilters,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Invoice>, i64), sqlx::Error> {
    let apply_where = |qb: &mut QueryBuilder<'_, Postgres>| {
        qb.push(" WHERE user_id = ").push_bind(user_id);

        if let Some(v) = filters.issuer_taxpayer_id.clone() {
            qb.push(" AND issuer_taxpayer_id = ").push_bind(v);
        }
        if let Some(v) = filters.receiver_taxpayer_id.clone() {
            qb.push(" AND receiver_taxpayer_id = ").push_bind(v);
        }
        if let Some(v) = filters.invoice_type.clone() {
            qb.push(" AND invoice_type::TEXT = ").push_bind(v);
        }
        if let Some(v) = filters.invoice_status.clone() {
            qb.push(" AND invoice_status::TEXT = ").push_bind(v);
        }
        if let Some(v) = filters.has_xml {
            qb.push(if v { " AND xml_file_id IS NOT NULL" } else { " AND xml_file_id IS NULL" });
        }
        if let Some(v) = filters.has_pdf {
            qb.push(if v { " AND pdf_file_id IS NOT NULL" } else { " AND pdf_file_id IS NULL" });
        }
        if let Some(v) = filters.uuid.clone() {
            qb.push(" AND uuid::TEXT = ").push_bind(v);
        }
        if let Some(v) = filters.fiscal_id.clone() {
            qb.push(" AND fiscal_id = ").push_bind(v);
        }
        if let Some(v) = &filters.issuer_name {
            qb.push(" AND issuer_name ILIKE ").push_bind(format!("%{v}%"));
        }
        if let Some(v) = &filters.receiver_name {
            qb.push(" AND receiver_name ILIKE ").push_bind(format!("%{v}%"));
        }
        if let Some(v) = filters.version.clone() {
            qb.push(" AND version = ").push_bind(v);
        }
        if let Some(v) = filters.series.clone() {
            qb.push(" AND series = ").push_bind(v);
        }
        if let Some(v) = filters.payment_form.clone() {
            qb.push(" AND payment_form = ").push_bind(v);
        }
        if let Some(v) = filters.currency.clone() {
            qb.push(" AND currency = ").push_bind(v);
        }
        if let Some(v) = filters.export.clone() {
            qb.push(" AND export = ").push_bind(v);
        }
        if let Some(v) = filters.payment_method.clone() {
            qb.push(" AND payment_method = ").push_bind(v);
        }
        if let Some(v) = filters.issue_place.clone() {
            qb.push(" AND issue_place = ").push_bind(v);
        }
        if let Some(v) = filters.cfdi_use.clone() {
            qb.push(" AND cfdi_use = ").push_bind(v);
        }
        if let Some(v) = filters.issuer_fiscal_regime.clone() {
            qb.push(" AND issuer_fiscal_regime = ").push_bind(v);
        }
        if let Some(v) = filters.recipient_fiscal_regime.clone() {
            qb.push(" AND recipient_fiscal_regime = ").push_bind(v);
        }
        if let Some(v) = filters.parsed {
            qb.push(" AND parsed = ").push_bind(v);
        }
        if let Some(v) = filters.issuer_id {
            qb.push(" AND issuer_id = ").push_bind(v);
        }
        if let Some(v) = filters.receiver_id {
            qb.push(" AND receiver_id = ").push_bind(v);
        }
        if let Some(v) = filters.issued_from {
            qb.push(" AND issued_at >= ").push_bind(v);
        }
        if let Some(v) = filters.issued_to {
            qb.push(" AND issued_at <= ").push_bind(v);
        }
        if let Some(v) = filters.total_min {
            qb.push(" AND total >= ").push_bind(v);
        }
        if let Some(v) = filters.total_max {
            qb.push(" AND total <= ").push_bind(v);
        }
        if let Some(v) = filters.taxpayer_id {
            qb.push(" AND (issuer_id = ")
                .push_bind(v)
                .push(" OR receiver_id = ")
                .push_bind(v)
                .push(")");
        }
    };

    let mut count_qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM invoices");
    apply_where(&mut count_qb);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut select_qb: QueryBuilder<Postgres> =
        QueryBuilder::new(format!("SELECT {SELECT_COLUMNS} FROM invoices"));
    apply_where(&mut select_qb);
    select_qb
        .push(" ORDER BY issued_at DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows: Vec<Invoice> = select_qb.build_query_as().fetch_all(pool).await?;

    Ok((rows, total))
}

pub async fn list_with_xml_for_user(
    pool: &PgPool,
    user_id: i32,
    force: bool,
) -> Result<Vec<Invoice>, sqlx::Error> {
    let extra = if force {
        ""
    } else {
        " AND (parsed IS NULL OR parsed = FALSE)"
    };
    sqlx::query_as::<_, Invoice>(
        &format!(
            "SELECT {SELECT_COLUMNS} FROM invoices WHERE user_id = $1 AND xml_file_id IS NOT NULL{extra}"
        ),
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn set_file_id(
    pool: &PgPool,
    id: i32,
    extension: &str,
    file_id: i32,
) -> Result<(), sqlx::Error> {
    let query = match extension {
        "xml" => "UPDATE invoices SET xml_file_id = $2 WHERE id = $1",
        "pdf" => "UPDATE invoices SET pdf_file_id = $2 WHERE id = $1",
        _ => return Ok(()),
    };
    sqlx::query(query)
        .bind(id)
        .bind(file_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_parse_result(pool: &PgPool, id: i32, data: ParsedData) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE invoices SET
            parsed = TRUE, parsing_error = NULL,
            issuer_id = $2, receiver_id = $3,
            invoice_type = $4::invoice_type_enum,
            version = $5, series = $6, payment_form = $7, payment_conditions = $8,
            subtotal = $9, discount = $10, currency = $11, exchange_rate = $12,
            export = $13, payment_method = $14, issue_place = $15,
            certificate_number = $16, cfdi_use = $17,
            issuer_fiscal_regime = $18, recipient_fiscal_regime = $19
         WHERE id = $1",
    )
    .bind(id)
    .bind(data.issuer_id)
    .bind(data.receiver_id)
    .bind(data.invoice_type)
    .bind(data.version)
    .bind(data.series)
    .bind(data.payment_form)
    .bind(data.payment_conditions)
    .bind(data.subtotal)
    .bind(data.discount)
    .bind(data.currency)
    .bind(data.exchange_rate)
    .bind(data.export)
    .bind(data.payment_method)
    .bind(data.issue_place)
    .bind(data.certificate_number)
    .bind(data.cfdi_use)
    .bind(data.issuer_fiscal_regime)
    .bind(data.recipient_fiscal_regime)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_parse_error(pool: &PgPool, id: i32, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invoices SET parsed = FALSE, parsing_error = $2 WHERE id = $1")
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;
    Ok(())
}
