use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
pub struct File {
    pub id: i32,
    pub user_id: i32,
    pub s3_key: String,
    pub extension: String,
    pub created_at: DateTime<Utc>,
}

pub async fn create(
    pool: &PgPool,
    user_id: i32,
    s3_key: &str,
    extension: &str,
) -> Result<File, sqlx::Error> {
    sqlx::query_as::<_, File>(
        "INSERT INTO files (user_id, s3_key, extension)
         VALUES ($1, $2, $3)
         RETURNING id, user_id, s3_key, extension, created_at",
    )
    .bind(user_id)
    .bind(s3_key)
    .bind(extension)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> Result<Option<File>, sqlx::Error> {
    sqlx::query_as::<_, File>(
        "SELECT id, user_id, s3_key, extension, created_at FROM files WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
