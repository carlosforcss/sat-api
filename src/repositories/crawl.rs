use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow, Clone)]
pub struct Crawl {
    pub id: i32,
    pub user_id: i32,
    pub link_id: Option<i32>,
    pub crawl_type: String,
    pub status: String,
    pub params: serde_json::Value,
    pub response_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    link_id: i32,
    crawl_type: &str,
    params: serde_json::Value,
) -> Result<Crawl, sqlx::Error> {
    sqlx::query_as::<_, Crawl>(
        "INSERT INTO crawls (user_id, link_id, crawl_type, params)
         VALUES ($1, $2, $3::crawl_type, $4)
         RETURNING id, user_id, link_id, crawl_type::TEXT, status::TEXT, params,
                   response_message, started_at, finished_at, created_at, updated_at",
    )
    .bind(user_id)
    .bind(link_id)
    .bind(crawl_type)
    .bind(params)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Crawl>, sqlx::Error> {
    sqlx::query_as::<_, Crawl>(
        "SELECT id, user_id, link_id, crawl_type::TEXT, status::TEXT, params,
                response_message, started_at, finished_at, created_at, updated_at
         FROM crawls WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn set_running(pool: &PgPool, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE crawls SET status = 'RUNNING'::crawl_status, started_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_finished(
    pool: &PgPool,
    id: i32,
    status: &str,
    response_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE crawls SET status = $2::crawl_status, response_message = $3, finished_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(response_message)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_for_user(
    pool: &PgPool,
    user_id: i32,
    link_id_filter: Option<i32>,
    crawl_type_filter: Option<&str>,
    status_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Crawl>, i64), sqlx::Error> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM crawls
         WHERE user_id = $1
           AND ($2::INT IS NULL OR link_id = $2)
           AND ($3::TEXT IS NULL OR crawl_type::TEXT = $3)
           AND ($4::TEXT IS NULL OR status::TEXT = $4)",
    )
    .bind(user_id)
    .bind(link_id_filter)
    .bind(crawl_type_filter)
    .bind(status_filter)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, Crawl>(
        "SELECT id, user_id, link_id, crawl_type::TEXT, status::TEXT,
                params, response_message, started_at, finished_at,
                created_at, updated_at
         FROM crawls
         WHERE user_id = $1
           AND ($2::INT IS NULL OR link_id = $2)
           AND ($3::TEXT IS NULL OR crawl_type::TEXT = $3)
           AND ($4::TEXT IS NULL OR status::TEXT = $4)
         ORDER BY created_at DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(user_id)
    .bind(link_id_filter)
    .bind(crawl_type_filter)
    .bind(status_filter)
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
) -> Result<Option<Crawl>, sqlx::Error> {
    sqlx::query_as::<_, Crawl>(
        "SELECT id, user_id, link_id, crawl_type::TEXT, status::TEXT,
                params, response_message, started_at, finished_at,
                created_at, updated_at
         FROM crawls
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
