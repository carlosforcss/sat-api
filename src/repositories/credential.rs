use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
pub struct Credential {
    pub id: i32,
    pub user_id: i32,
    pub rfc: String,
    pub cred_type: String,
    pub status: String,
    pub password_hash: String,
    pub cer_path: Option<String>,
    pub key_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    rfc: &str,
    cred_type: &str,
    password_hash: &str,
    cer_path: Option<&str>,
    key_path: Option<&str>,
) -> Result<Credential, sqlx::Error> {
    sqlx::query_as::<_, Credential>(
        "INSERT INTO credentials (user_id, rfc, cred_type, password_hash, cer_path, key_path)
         VALUES ($1, $2, $3::credential_type, $4, $5, $6)
         RETURNING id, user_id, rfc, cred_type::TEXT, status::TEXT, password_hash, cer_path, key_path, created_at, updated_at",
    )
    .bind(user_id)
    .bind(rfc)
    .bind(cred_type)
    .bind(password_hash)
    .bind(cer_path)
    .bind(key_path)
    .fetch_one(pool)
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
    sqlx::query_as::<_, Credential>(
        "SELECT id, user_id, rfc, cred_type::TEXT, status::TEXT, password_hash, cer_path, key_path, created_at, updated_at
         FROM credentials WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}
