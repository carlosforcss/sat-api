use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow, Clone)]
pub struct Invoice {
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
    link_id: i32,
    uuid: &str,
    fiscal_id: &str,
    issuer_taxpayer_id: &str,
    issuer_name: &str,
    receiver_taxpayer_id: &str,
    receiver_name: &str,
    issued_at: &str,
    certified_at: &str,
    total: &str,
    invoice_type: &str,
    invoice_status: &str,
    download_path: &str,
) -> Result<Invoice, sqlx::Error> {
    sqlx::query_as::<_, Invoice>(
        "INSERT INTO invoices (link_id, uuid, fiscal_id, issuer_taxpayer_id, issuer_name,
                               receiver_taxpayer_id, receiver_name, issued_at, certified_at,
                               total, invoice_type, invoice_status, download_path)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (uuid, link_id) DO UPDATE SET
             fiscal_id            = EXCLUDED.fiscal_id,
             issuer_taxpayer_id   = EXCLUDED.issuer_taxpayer_id,
             issuer_name          = EXCLUDED.issuer_name,
             receiver_taxpayer_id = EXCLUDED.receiver_taxpayer_id,
             receiver_name        = EXCLUDED.receiver_name,
             issued_at            = EXCLUDED.issued_at,
             certified_at         = EXCLUDED.certified_at,
             total                = EXCLUDED.total,
             invoice_type         = EXCLUDED.invoice_type,
             invoice_status       = EXCLUDED.invoice_status,
             download_path        = EXCLUDED.download_path
         RETURNING id, link_id, uuid, fiscal_id, issuer_taxpayer_id, issuer_name,
                   receiver_taxpayer_id, receiver_name, issued_at, certified_at, total,
                   invoice_type, invoice_status, download_path, xml_file_id, pdf_file_id,
                   created_at",
    )
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
    .bind(download_path)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id_for_user(
    pool: &PgPool,
    id: i32,
    user_id: i32,
) -> Result<Option<Invoice>, sqlx::Error> {
    sqlx::query_as::<_, Invoice>(
        "SELECT invoices.id, invoices.link_id, invoices.uuid, invoices.fiscal_id,
                invoices.issuer_taxpayer_id, invoices.issuer_name,
                invoices.receiver_taxpayer_id, invoices.receiver_name,
                invoices.issued_at, invoices.certified_at, invoices.total,
                invoices.invoice_type, invoices.invoice_status,
                invoices.download_path, invoices.xml_file_id, invoices.pdf_file_id,
                invoices.created_at
         FROM invoices
         JOIN links ON links.id = invoices.link_id
         WHERE invoices.id = $1 AND links.user_id = $2",
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
         JOIN links ON links.id = invoices.link_id
         WHERE links.user_id = $1
           AND ($2::TEXT IS NULL OR invoices.issuer_taxpayer_id = $2)
           AND ($3::TEXT IS NULL OR invoices.receiver_taxpayer_id = $3)
           AND ($4::TEXT IS NULL OR invoices.invoice_type = $4)
           AND ($5::TEXT IS NULL OR invoices.invoice_status = $5)
           AND ($6::BOOL IS NULL OR ($6 = TRUE AND invoices.xml_file_id IS NOT NULL) OR ($6 = FALSE AND invoices.xml_file_id IS NULL))
           AND ($7::BOOL IS NULL OR ($7 = TRUE AND invoices.pdf_file_id IS NOT NULL) OR ($7 = FALSE AND invoices.pdf_file_id IS NULL))",
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
        "SELECT invoices.id, invoices.link_id, invoices.uuid, invoices.fiscal_id,
                invoices.issuer_taxpayer_id, invoices.issuer_name,
                invoices.receiver_taxpayer_id, invoices.receiver_name,
                invoices.issued_at, invoices.certified_at, invoices.total,
                invoices.invoice_type, invoices.invoice_status,
                invoices.download_path, invoices.xml_file_id, invoices.pdf_file_id,
                invoices.created_at
         FROM invoices
         JOIN links ON links.id = invoices.link_id
         WHERE links.user_id = $1
           AND ($2::TEXT IS NULL OR invoices.issuer_taxpayer_id = $2)
           AND ($3::TEXT IS NULL OR invoices.receiver_taxpayer_id = $3)
           AND ($4::TEXT IS NULL OR invoices.invoice_type = $4)
           AND ($5::TEXT IS NULL OR invoices.invoice_status = $5)
           AND ($6::BOOL IS NULL OR ($6 = TRUE AND invoices.xml_file_id IS NOT NULL) OR ($6 = FALSE AND invoices.xml_file_id IS NULL))
           AND ($7::BOOL IS NULL OR ($7 = TRUE AND invoices.pdf_file_id IS NOT NULL) OR ($7 = FALSE AND invoices.pdf_file_id IS NULL))
         ORDER BY invoices.issued_at DESC
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
