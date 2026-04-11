use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;

use crate::repositories::credential::{self, Credential};

pub enum CredentialError {
    AlreadyExists,
    Internal,
}

impl IntoResponse for CredentialError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            CredentialError::AlreadyExists => (StatusCode::CONFLICT, "credential already exists"),
            CredentialError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub async fn create_ciec(
    pool: &PgPool,
    user_id: i32,
    rfc: &str,
    password: &str,
) -> Result<Credential, CredentialError> {
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|_| CredentialError::Internal)?;

    credential::create(pool, user_id, rfc, "CIEC", &password_hash, None, None)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("credentials_user_id_rfc_cred_type_key") {
                    return CredentialError::AlreadyExists;
                }
            }
            CredentialError::Internal
        })
}

pub async fn create_fiel(
    pool: &PgPool,
    upload_path: &str,
    user_id: i32,
    rfc: &str,
    password: &str,
    cer_bytes: Vec<u8>,
    key_bytes: Vec<u8>,
) -> Result<Credential, CredentialError> {
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|_| CredentialError::Internal)?;

    let dir = format!("{}/{}/{}", upload_path, user_id, rfc);
    tokio::fs::create_dir_all(&dir).await.map_err(|_| CredentialError::Internal)?;

    let cer_path = format!("{}/cert.cer", dir);
    let key_path = format!("{}/private.key", dir);

    let mut cer_file = tokio::fs::File::create(&cer_path).await.map_err(|_| CredentialError::Internal)?;
    cer_file.write_all(&cer_bytes).await.map_err(|_| CredentialError::Internal)?;

    let mut key_file = tokio::fs::File::create(&key_path).await.map_err(|_| CredentialError::Internal)?;
    key_file.write_all(&key_bytes).await.map_err(|_| CredentialError::Internal)?;

    credential::create(pool, user_id, rfc, "FIEL", &password_hash, Some(&cer_path), Some(&key_path))
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("credentials_user_id_rfc_cred_type_key") {
                    return CredentialError::AlreadyExists;
                }
            }
            CredentialError::Internal
        })
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, CredentialError> {
    credential::delete(pool, id, user_id)
        .await
        .map_err(|_| CredentialError::Internal)
}

pub async fn list(pool: &PgPool, user_id: i32) -> Result<Vec<Credential>, CredentialError> {
    credential::list_by_user(pool, user_id)
        .await
        .map_err(|_| CredentialError::Internal)
}
