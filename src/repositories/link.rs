use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow, Clone)]
pub struct Link {
    pub id: i32,
    pub user_id: i32,
    pub credential_id: i32,
    pub taxpayer_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const SELECT: &str =
    "SELECT id, user_id, credential_id, taxpayer_id, status::TEXT, created_at, updated_at FROM links";

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    credential_id: i32,
    taxpayer_id: &str,
) -> Result<Link, sqlx::Error> {
    sqlx::query_as::<_, Link>(
        "INSERT INTO links (user_id, credential_id, taxpayer_id)
         VALUES ($1, $2, $3)
         RETURNING id, user_id, credential_id, taxpayer_id, status::TEXT, created_at, updated_at",
    )
    .bind(user_id)
    .bind(credential_id)
    .bind(taxpayer_id)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Link>, sqlx::Error> {
    sqlx::query_as::<_, Link>(&format!("{SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_id_and_user(
    pool: &PgPool,
    id: i32,
    user_id: i32,
) -> Result<Option<Link>, sqlx::Error> {
    sqlx::query_as::<_, Link>(&format!("{SELECT} WHERE id = $1 AND user_id = $2"))
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn list_by_user(pool: &PgPool, user_id: i32) -> Result<Vec<Link>, sqlx::Error> {
    sqlx::query_as::<_, Link>(&format!(
        "{SELECT} WHERE user_id = $1 ORDER BY created_at DESC"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM links WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_status(pool: &PgPool, id: i32, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE links SET status = $2::link_status, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}
