use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

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
    // ranges
    pub issued_from: Option<DateTime<Utc>>,
    pub issued_to: Option<DateTime<Utc>>,
    pub total_min: Option<f64>,
    pub total_max: Option<f64>,
}

pub struct ParsedData {
    pub issuer_id: Option<i32>,
    pub receiver_id: Option<i32>,
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
    let where_clause = "WHERE user_id = $1
        AND ($2::TEXT IS NULL OR issuer_taxpayer_id = $2)
        AND ($3::TEXT IS NULL OR receiver_taxpayer_id = $3)
        AND ($4::TEXT IS NULL OR invoice_type::TEXT = $4)
        AND ($5::TEXT IS NULL OR invoice_status::TEXT = $5)
        AND ($6::BOOL IS NULL OR ($6 = TRUE AND xml_file_id IS NOT NULL) OR ($6 = FALSE AND xml_file_id IS NULL))
        AND ($7::BOOL IS NULL OR ($7 = TRUE AND pdf_file_id IS NOT NULL) OR ($7 = FALSE AND pdf_file_id IS NULL))
        AND ($8::TEXT IS NULL OR uuid::TEXT = $8)
        AND ($9::TEXT IS NULL OR fiscal_id = $9)
        AND ($10::TEXT IS NULL OR issuer_name ILIKE '%' || $10 || '%')
        AND ($11::TEXT IS NULL OR receiver_name ILIKE '%' || $11 || '%')
        AND ($12::TEXT IS NULL OR version = $12)
        AND ($13::TEXT IS NULL OR series = $13)
        AND ($14::TEXT IS NULL OR payment_form = $14)
        AND ($15::TEXT IS NULL OR currency = $15)
        AND ($16::TEXT IS NULL OR export = $16)
        AND ($17::TEXT IS NULL OR payment_method = $17)
        AND ($18::TEXT IS NULL OR issue_place = $18)
        AND ($19::TEXT IS NULL OR cfdi_use = $19)
        AND ($20::TEXT IS NULL OR issuer_fiscal_regime = $20)
        AND ($21::TEXT IS NULL OR recipient_fiscal_regime = $21)
        AND ($22::BOOL IS NULL OR parsed = $22)
        AND ($23::INT IS NULL OR issuer_id = $23)
        AND ($24::INT IS NULL OR receiver_id = $24)
        AND ($25::TIMESTAMPTZ IS NULL OR issued_at >= $25)
        AND ($26::TIMESTAMPTZ IS NULL OR issued_at <= $26)
        AND ($27::FLOAT8 IS NULL OR total >= $27)
        AND ($28::FLOAT8 IS NULL OR total <= $28)";

    macro_rules! bind_filters {
        ($q:expr, $f:expr) => {
            $q.bind(user_id)
                .bind(&$f.issuer_taxpayer_id)
                .bind(&$f.receiver_taxpayer_id)
                .bind(&$f.invoice_type)
                .bind(&$f.invoice_status)
                .bind($f.has_xml)
                .bind($f.has_pdf)
                .bind(&$f.uuid)
                .bind(&$f.fiscal_id)
                .bind(&$f.issuer_name)
                .bind(&$f.receiver_name)
                .bind(&$f.version)
                .bind(&$f.series)
                .bind(&$f.payment_form)
                .bind(&$f.currency)
                .bind(&$f.export)
                .bind(&$f.payment_method)
                .bind(&$f.issue_place)
                .bind(&$f.cfdi_use)
                .bind(&$f.issuer_fiscal_regime)
                .bind(&$f.recipient_fiscal_regime)
                .bind($f.parsed)
                .bind($f.issuer_id)
                .bind($f.receiver_id)
                .bind($f.issued_from)
                .bind($f.issued_to)
                .bind($f.total_min)
                .bind($f.total_max)
        };
    }

    let total: i64 = bind_filters!(
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM invoices {where_clause}")),
        filters
    )
    .fetch_one(pool)
    .await?;

    let rows = bind_filters!(
        sqlx::query_as::<_, Invoice>(&format!(
            "SELECT {SELECT_COLUMNS} FROM invoices {where_clause} ORDER BY issued_at DESC LIMIT $29 OFFSET $30"
        )),
        filters
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

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
            version = $4, series = $5, payment_form = $6, payment_conditions = $7,
            subtotal = $8, discount = $9, currency = $10, exchange_rate = $11,
            export = $12, payment_method = $13, issue_place = $14,
            certificate_number = $15, cfdi_use = $16,
            issuer_fiscal_regime = $17, recipient_fiscal_regime = $18
         WHERE id = $1",
    )
    .bind(id)
    .bind(data.issuer_id)
    .bind(data.receiver_id)
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
