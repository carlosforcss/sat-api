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
    pub created_at: DateTime<Utc>,
}

pub struct InvoiceFilters {
    pub issuer_taxpayer_id: Option<String>,
    pub receiver_taxpayer_id: Option<String>,
    pub invoice_type: Option<String>,
    pub invoice_status: Option<String>,
    pub has_xml: Option<bool>,
    pub has_pdf: Option<bool>,
}

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
         RETURNING id, user_id, link_id, uuid::TEXT, fiscal_id, issuer_taxpayer_id, issuer_name,
                   receiver_taxpayer_id, receiver_name, issued_at, certified_at, total::FLOAT8,
                   invoice_type::TEXT, invoice_status::TEXT, xml_file_id, pdf_file_id, created_at",
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
    sqlx::query_as::<_, Invoice>(
        "SELECT id, user_id, link_id, uuid::TEXT, fiscal_id, issuer_taxpayer_id, issuer_name,
                receiver_taxpayer_id, receiver_name, issued_at, certified_at, total::FLOAT8,
                invoice_type::TEXT, invoice_status::TEXT, xml_file_id, pdf_file_id, created_at
         FROM invoices
         WHERE uuid = $1::UUID AND user_id = $2",
    )
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
    sqlx::query_as::<_, Invoice>(
        "SELECT id, user_id, link_id, uuid::TEXT, fiscal_id, issuer_taxpayer_id, issuer_name,
                receiver_taxpayer_id, receiver_name, issued_at, certified_at, total::FLOAT8,
                invoice_type::TEXT, invoice_status::TEXT, xml_file_id, pdf_file_id, created_at
         FROM invoices
         WHERE id = $1 AND user_id = $2",
    )
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
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM invoices
         WHERE user_id = $1
           AND ($2::TEXT IS NULL OR issuer_taxpayer_id = $2)
           AND ($3::TEXT IS NULL OR receiver_taxpayer_id = $3)
           AND ($4::TEXT IS NULL OR invoice_type::TEXT = $4)
           AND ($5::TEXT IS NULL OR invoice_status::TEXT = $5)
           AND ($6::BOOL IS NULL OR ($6 = TRUE AND xml_file_id IS NOT NULL) OR ($6 = FALSE AND xml_file_id IS NULL))
           AND ($7::BOOL IS NULL OR ($7 = TRUE AND pdf_file_id IS NOT NULL) OR ($7 = FALSE AND pdf_file_id IS NULL))",
    )
    .bind(user_id)
    .bind(&filters.issuer_taxpayer_id)
    .bind(&filters.receiver_taxpayer_id)
    .bind(&filters.invoice_type)
    .bind(&filters.invoice_status)
    .bind(filters.has_xml)
    .bind(filters.has_pdf)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, Invoice>(
        "SELECT id, user_id, link_id, uuid::TEXT, fiscal_id, issuer_taxpayer_id, issuer_name,
                receiver_taxpayer_id, receiver_name, issued_at, certified_at, total::FLOAT8,
                invoice_type::TEXT, invoice_status::TEXT, xml_file_id, pdf_file_id, created_at
         FROM invoices
         WHERE user_id = $1
           AND ($2::TEXT IS NULL OR issuer_taxpayer_id = $2)
           AND ($3::TEXT IS NULL OR receiver_taxpayer_id = $3)
           AND ($4::TEXT IS NULL OR invoice_type::TEXT = $4)
           AND ($5::TEXT IS NULL OR invoice_status::TEXT = $5)
           AND ($6::BOOL IS NULL OR ($6 = TRUE AND xml_file_id IS NOT NULL) OR ($6 = FALSE AND xml_file_id IS NULL))
           AND ($7::BOOL IS NULL OR ($7 = TRUE AND pdf_file_id IS NOT NULL) OR ($7 = FALSE AND pdf_file_id IS NULL))
         ORDER BY issued_at DESC
         LIMIT $8 OFFSET $9",
    )
    .bind(user_id)
    .bind(filters.issuer_taxpayer_id)
    .bind(filters.receiver_taxpayer_id)
    .bind(filters.invoice_type)
    .bind(filters.invoice_status)
    .bind(filters.has_xml)
    .bind(filters.has_pdf)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((rows, total))
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
