use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow, Clone)]
pub struct Taxpayer {
    pub id: i32,
    pub user_id: i32,
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

pub struct TaxpayerData {
    pub taxpayer_id: String,
    pub name: String,
    pub cfdi_use: Option<String>,
    pub fiscal_domicile: Option<String>,
    pub fiscal_regime: Option<String>,
    pub foreign_tax_id: Option<String>,
    pub tax_residence: Option<String>,
    pub last_seen_at: DateTime<Utc>,
}

pub struct TaxpayerFilters {
    pub taxpayer_id: Option<String>,
    pub name: Option<String>,
}

pub async fn upsert(pool: &PgPool, user_id: i32, data: TaxpayerData) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO taxpayers (user_id, taxpayer_id, name, cfdi_use, fiscal_domicile, fiscal_regime, foreign_tax_id, tax_residence, last_seen_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (user_id, taxpayer_id) DO UPDATE SET
             name = CASE
                 WHEN EXCLUDED.last_seen_at > taxpayers.last_seen_at
                 THEN EXCLUDED.name
                 ELSE taxpayers.name
             END,
             cfdi_use = CASE
                 WHEN EXCLUDED.cfdi_use IS NOT NULL
                  AND (EXCLUDED.last_seen_at > taxpayers.last_seen_at OR taxpayers.cfdi_use IS NULL)
                 THEN EXCLUDED.cfdi_use
                 ELSE taxpayers.cfdi_use
             END,
             fiscal_domicile = CASE
                 WHEN EXCLUDED.fiscal_domicile IS NOT NULL
                  AND (EXCLUDED.last_seen_at > taxpayers.last_seen_at OR taxpayers.fiscal_domicile IS NULL)
                 THEN EXCLUDED.fiscal_domicile
                 ELSE taxpayers.fiscal_domicile
             END,
             fiscal_regime = CASE
                 WHEN EXCLUDED.fiscal_regime IS NOT NULL
                  AND (EXCLUDED.last_seen_at > taxpayers.last_seen_at OR taxpayers.fiscal_regime IS NULL)
                 THEN EXCLUDED.fiscal_regime
                 ELSE taxpayers.fiscal_regime
             END,
             foreign_tax_id = CASE
                 WHEN EXCLUDED.foreign_tax_id IS NOT NULL
                  AND (EXCLUDED.last_seen_at > taxpayers.last_seen_at OR taxpayers.foreign_tax_id IS NULL)
                 THEN EXCLUDED.foreign_tax_id
                 ELSE taxpayers.foreign_tax_id
             END,
             tax_residence = CASE
                 WHEN EXCLUDED.tax_residence IS NOT NULL
                  AND (EXCLUDED.last_seen_at > taxpayers.last_seen_at OR taxpayers.tax_residence IS NULL)
                 THEN EXCLUDED.tax_residence
                 ELSE taxpayers.tax_residence
             END,
             last_seen_at = GREATEST(taxpayers.last_seen_at, EXCLUDED.last_seen_at)
         RETURNING id",
    )
    .bind(user_id)
    .bind(data.taxpayer_id)
    .bind(data.name)
    .bind(data.cfdi_use)
    .bind(data.fiscal_domicile)
    .bind(data.fiscal_regime)
    .bind(data.foreign_tax_id)
    .bind(data.tax_residence)
    .bind(data.last_seen_at)
    .fetch_one(pool)
    .await
}

pub async fn list_for_user(
    pool: &PgPool,
    user_id: i32,
    filters: TaxpayerFilters,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Taxpayer>, i64), sqlx::Error> {
    let where_clause = "WHERE user_id = $1
        AND ($2::TEXT IS NULL OR taxpayer_id = $2)
        AND ($3::TEXT IS NULL OR name ILIKE '%' || $3 || '%')";

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM taxpayers {where_clause}"))
        .bind(user_id)
        .bind(&filters.taxpayer_id)
        .bind(&filters.name)
        .fetch_one(pool)
        .await?;

    let rows = sqlx::query_as::<_, Taxpayer>(&format!(
        "SELECT id, user_id, taxpayer_id, name, cfdi_use, fiscal_domicile, fiscal_regime,
                    foreign_tax_id, tax_residence, last_seen_at, created_at
             FROM taxpayers {where_clause}
             ORDER BY name ASC LIMIT $4 OFFSET $5"
    ))
    .bind(user_id)
    .bind(filters.taxpayer_id)
    .bind(filters.name)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((rows, total))
}

pub async fn find_by_id_for_user(
    pool: &PgPool,
    id: i32,
    user_id: i32,
) -> Result<Option<Taxpayer>, sqlx::Error> {
    sqlx::query_as::<_, Taxpayer>(
        "SELECT id, user_id, taxpayer_id, name, cfdi_use, fiscal_domicile, fiscal_regime,
                foreign_tax_id, tax_residence, last_seen_at, created_at
         FROM taxpayers WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
