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
    rfc: &str,
    password: &str,
) -> Result<Credential, CredentialError> {
    let encrypted = crate::crypto::encrypt(password).map_err(|e| {
        tracing::error!("failed to encrypt CIEC password: {e}");
        CredentialError::Internal
    })?;

    let cred = credential::create(pool, user_id, rfc, "CIEC", &encrypted, None, None)
        .await
        .map_err(|e| {
            tracing::error!("failed to insert CIEC credential: {e}");
            CredentialError::Internal
        })?;

    spawn_validate_crawl(pool, cred.id).await;

    Ok(cred)
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
    let encrypted = crate::crypto::encrypt(password).map_err(|e| {
        tracing::error!("failed to encrypt FIEL password: {e}");
        CredentialError::Internal
    })?;

    let dir = format!("{}/{}/{}", upload_path, user_id, rfc);
    tokio::fs::create_dir_all(&dir).await.map_err(|_| CredentialError::Internal)?;

    let cer_path = format!("{}/cert.cer", dir);
    let key_path = format!("{}/private.key", dir);

    let mut cer_file = tokio::fs::File::create(&cer_path).await.map_err(|_| CredentialError::Internal)?;
    cer_file.write_all(&cer_bytes).await.map_err(|_| CredentialError::Internal)?;

    let mut key_file = tokio::fs::File::create(&key_path).await.map_err(|_| CredentialError::Internal)?;
    key_file.write_all(&key_bytes).await.map_err(|_| CredentialError::Internal)?;

    let cred = credential::create(pool, user_id, rfc, "FIEL", &encrypted, Some(&cer_path), Some(&key_path))
        .await
        .map_err(|e| {
            tracing::error!("failed to insert FIEL credential: {e}");
            CredentialError::Internal
        })?;

    spawn_validate_crawl(pool, cred.id).await;

    Ok(cred)
}

async fn spawn_validate_crawl(pool: &PgPool, credential_id: i32) {
    match crate::repositories::crawl::create(
        pool,
        credential_id,
        "VALIDATE_CREDENTIALS",
        serde_json::json!({}),
    )
    .await
    {
        Ok(crawl) => {
            let pool_clone = pool.clone();
            let crawl_id = crawl.id;
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("failed to build crawler runtime");
                rt.block_on(crate::crawlers::run_crawl(&pool_clone, crawl_id));
            });
        }
        Err(e) => {
            tracing::error!(
                "failed to create validation crawl for credential {credential_id}: {e}"
            );
        }
    }
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
