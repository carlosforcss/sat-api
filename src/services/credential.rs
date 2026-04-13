use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;

use crate::repositories::credential::{self, Credential};

pub enum CredentialError {
    Internal,
}

impl IntoResponse for CredentialError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "internal error" })),
        )
            .into_response()
    }
}

pub async fn create_ciec(
    pool: &PgPool,
    user_id: i32,
    taxpayer_id: &str,
    password: &str,
) -> Result<Credential, CredentialError> {
    let encrypted = crate::crypto::encrypt(password).map_err(|e| {
        tracing::error!("failed to encrypt CIEC password: {e}");
        CredentialError::Internal
    })?;

    credential::create(pool, user_id, taxpayer_id, "CIEC", &encrypted, None, None)
        .await
        .map_err(|e| {
            tracing::error!("failed to insert CIEC credential: {e}");
            CredentialError::Internal
        })
}

pub async fn create_fiel(
    pool: &PgPool,
    upload_path: &str,
    user_id: i32,
    taxpayer_id: &str,
    password: &str,
    cer_bytes: Vec<u8>,
    key_bytes: Vec<u8>,
) -> Result<Credential, CredentialError> {
    let encrypted = crate::crypto::encrypt(password).map_err(|e| {
        tracing::error!("failed to encrypt FIEL password: {e}");
        CredentialError::Internal
    })?;

    let dir = format!("{}/{}/{}", upload_path, user_id, taxpayer_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|_| CredentialError::Internal)?;

    let cer_path = format!("{}/cert.cer", dir);
    let key_path = format!("{}/private.key", dir);

    let mut cer_file = tokio::fs::File::create(&cer_path)
        .await
        .map_err(|_| CredentialError::Internal)?;
    cer_file
        .write_all(&cer_bytes)
        .await
        .map_err(|_| CredentialError::Internal)?;

    let mut key_file = tokio::fs::File::create(&key_path)
        .await
        .map_err(|_| CredentialError::Internal)?;
    key_file
        .write_all(&key_bytes)
        .await
        .map_err(|_| CredentialError::Internal)?;

    credential::create(
        pool,
        user_id,
        taxpayer_id,
        "FIEL",
        &encrypted,
        Some(&cer_path),
        Some(&key_path),
    )
    .await
    .map_err(|e| {
        tracing::error!("failed to insert FIEL credential: {e}");
        CredentialError::Internal
    })
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, CredentialError> {
    credential::delete(pool, id, user_id)
        .await
        .map_err(|_| CredentialError::Internal)
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Credential>, i64), CredentialError> {
    let per_page = per_page.clamp(1, 100);
    let page = page.max(1);
    let offset = (page - 1) * per_page;
    credential::list_by_user(pool, user_id, per_page, offset)
        .await
        .map_err(|_| CredentialError::Internal)
}
