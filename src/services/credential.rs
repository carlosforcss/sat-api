use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;

use crate::repositories::credential::Credential;
use crate::repositories::{crawl as crawl_repo, credential, link as link_repo};

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

    spawn_validate_crawl(pool, &cred);
    Ok(cred)
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

    spawn_validate_crawl(pool, &cred);
    Ok(cred)
}

fn spawn_validate_crawl(pool: &PgPool, cred: &Credential) {
    let pool = pool.clone();
    let user_id = cred.user_id;
    let credential_id = cred.id;
    let taxpayer_id = cred.taxpayer_id.clone();
    tokio::spawn(async move {
        let (link_id, crawl_params) =
            match link_repo::find_valid_by_user_and_taxpayer_id(&pool, user_id, &taxpayer_id).await
            {
                Ok(Some(valid_link)) => {
                    let old_credential_id = valid_link.credential_id;
                    if let Err(e) = link_repo::update_credential_and_status(
                        &pool,
                        valid_link.id,
                        credential_id,
                        "INVALID",
                    )
                    .await
                    {
                        tracing::error!(
                            "failed to update link {} for new credential: {e}",
                            valid_link.id
                        );
                        return;
                    }
                    (
                        valid_link.id,
                        serde_json::json!({ "old_credential_id": old_credential_id }),
                    )
                }
                Ok(None) => {
                    // No VALID link — upsert (creates or updates an INVALID link).
                    match link_repo::create(&pool, user_id, credential_id, &taxpayer_id).await {
                        Ok(link) => (link.id, serde_json::json!({})),
                        Err(e) => {
                            tracing::error!(
                                "failed to create link for credential {credential_id}: {e}"
                            );
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("failed to query link for credential {credential_id}: {e}");
                    return;
                }
            };

        match crawl_repo::create(&pool, link_id, "VALIDATE_CREDENTIALS", crawl_params).await {
            Ok(crawl) => crate::services::crawl::spawn(&pool, crawl.id),
            Err(e) => {
                tracing::error!("failed to create validation crawl for link {link_id}: {e}")
            }
        }
    });
}

pub async fn delete(pool: &PgPool, id: i32, user_id: i32) -> Result<bool, CredentialError> {
    credential::delete(pool, id, user_id).await.map_err(|e| {
        if is_fk_violation(&e) {
            CredentialError::InUse
        } else {
            CredentialError::Internal
        }
    })
}

fn is_fk_violation(e: &sqlx::Error) -> bool {
    matches!(
        e,
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23503")
    )
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
