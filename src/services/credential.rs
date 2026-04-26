use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;

use crate::repositories::credential::Credential;
use crate::repositories::credential;
use crate::storage::S3Storage;

pub enum CredentialError {
    Internal,
    InUse,
}

impl IntoResponse for CredentialError {
    fn into_response(self) -> axum::response::Response {
        match self {
            CredentialError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal error" })),
            )
                .into_response(),
            CredentialError::InUse => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "credential is in use by a link" })),
            )
                .into_response(),
        }
    }
}

pub async fn create_ciec(
    pool: &PgPool,
    storage: Arc<S3Storage>,
    user_id: i32,
    taxpayer_id: &str,
    password: &str,
) -> Result<Credential, CredentialError> {
    let encrypted = crate::crypto::encrypt(password).map_err(|e| {
        tracing::error!("failed to encrypt CIEC password: {e}");
        CredentialError::Internal
    })?;

    let cred = credential::create(pool, user_id, taxpayer_id, "CIEC", &encrypted, None, None)
        .await
        .map_err(|e| {
            tracing::error!("failed to insert CIEC credential: {e}");
            CredentialError::Internal
        })?;

    crate::reactor::on_credential_created(pool, storage, &cred);
    Ok(cred)
}

pub async fn create_fiel(
    pool: &PgPool,
    storage: Arc<S3Storage>,
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

    let cred = credential::create(
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
    })?;

    crate::reactor::on_credential_created(pool, storage, &cred);
    Ok(cred)
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, CredentialError> {
    credential::delete(pool, id, user_id).await.map_err(|e| {
        if crate::repositories::is_fk_violation(&e) {
            CredentialError::InUse
        } else {
            CredentialError::Internal
        }
    })
}

pub async fn list(
    pool: &PgPool,
    user_id: i32,
    page: i64,
    per_page: i64,
) -> Result<(Vec<Credential>, i64), CredentialError> {
    let (_, per_page, offset) = crate::services::paginate(page, per_page);
    credential::list_by_user(pool, user_id, per_page, offset)
        .await
        .map_err(|_| CredentialError::Internal)
}
