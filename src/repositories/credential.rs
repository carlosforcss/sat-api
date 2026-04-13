use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow, Clone)]
pub struct Credential {
    pub id: i32,
    pub user_id: i32,
    pub taxpayer_id: String,
    pub cred_type: String,
    pub status: String,
    pub password: String,
    pub cer_path: Option<String>,
    pub key_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const SELECT: &str = "SELECT id, user_id, taxpayer_id, cred_type::TEXT, status::TEXT, password, cer_path, key_path, created_at, updated_at FROM credentials";

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    taxpayer_id: &str,
    cred_type: &str,
    password: &str,
    cer_path: Option<&str>,
    key_path: Option<&str>,
) -> Result<Credential, sqlx::Error> {
    sqlx::query_as::<_, Credential>(
        "INSERT INTO credentials (user_id, taxpayer_id, cred_type, password, cer_path, key_path)
         VALUES ($1, $2, $3::credential_type, $4, $5, $6)
         RETURNING id, user_id, taxpayer_id, cred_type::TEXT, status::TEXT, password, cer_path, key_path, created_at, updated_at",
    )
    .bind(user_id)
    .bind(taxpayer_id)
    .bind(cred_type)
    .bind(password)
    .bind(cer_path)
    .bind(key_path)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<Credential>, sqlx::Error> {
    sqlx::query_as::<_, Credential>(&format!("{SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_id_and_user(
    pool: &PgPool,
    id: i32,
    user_id: i32,
) -> Result<Option<Credential>, sqlx::Error> {
    sqlx::query_as::<_, Credential>(&format!("{SELECT} WHERE id = $1 AND user_id = $2"))
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM credentials WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_by_user(pool: &PgPool, user_id: i32) -> Result<Vec<Credential>, sqlx::Error> {
    sqlx::query_as::<_, Credential>(&format!(
        "{SELECT} WHERE user_id = $1 ORDER BY created_at DESC"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn update_status(pool: &PgPool, id: i32, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE credentials SET status = $2::credential_status, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}
