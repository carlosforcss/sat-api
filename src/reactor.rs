use std::sync::Arc;

use sqlx::PgPool;

use crate::repositories::{crawl as crawl_repo, credential::Credential, link as link_repo};
use crate::storage::S3Storage;

/// Credential just created → set up link and kick off validation crawl.
pub fn on_credential_created(pool: &PgPool, storage: Arc<S3Storage>, cred: &Credential) {
    let pool = pool.clone();
    let user_id = cred.user_id;
    let credential_id = cred.id;
    let taxpayer_id = cred.taxpayer_id.clone();

    tracing::info!(credential_id, taxpayer_id, "reactor: credential_created");

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
                            credential_id,
                            link_id = valid_link.id,
                            "reactor: failed to update link for new credential: {e}"
                        );
                        return;
                    }
                    (
                        valid_link.id,
                        serde_json::json!({ "old_credential_id": old_credential_id }),
                    )
                }
                Ok(None) => {
                    match link_repo::create(&pool, user_id, credential_id, &taxpayer_id).await {
                        Ok(link) => (link.id, serde_json::json!({})),
                        Err(e) => {
                            tracing::error!(
                                credential_id,
                                "reactor: failed to create link: {e}"
                            );
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(credential_id, "reactor: failed to query link: {e}");
                    return;
                }
            };

        match crawl_repo::create(&pool, user_id, link_id, "VALIDATE_CREDENTIALS", crawl_params).await {
            Ok(crawl) => {
                tracing::info!(credential_id, link_id, crawl_id = crawl.id, "reactor: spawning VALIDATE_CREDENTIALS");
                crate::services::crawl::spawn(&pool, crawl.id, storage);
            }
            Err(e) => {
                tracing::error!(credential_id, link_id, "reactor: failed to create validation crawl: {e}");
            }
        }
    });
}

/// VALIDATE_CREDENTIALS crawl succeeded → mark link VALID, start both download crawls.
pub async fn on_validation_succeeded(
    pool: &PgPool,
    storage: Arc<S3Storage>,
    link_id: i32,
    user_id: i32,
) -> Result<(), String> {
    tracing::info!(link_id, user_id, "reactor: validation_succeeded → spawning DOWNLOAD_ISSUED + DOWNLOAD_RECEIVED");

    link_repo::update_status(pool, link_id, "VALID")
        .await
        .map_err(|e| e.to_string())?;

    for crawl_type in &["DOWNLOAD_ISSUED_INVOICES", "DOWNLOAD_RECEIVED_INVOICES"] {
        match crawl_repo::create(
            pool,
            user_id,
            link_id,
            crawl_type,
            serde_json::json!({ "start_date": "01/01/2017" }),
        )
        .await
        {
            Ok(c) => {
                tracing::info!(link_id, crawl_id = c.id, crawl_type, "reactor: spawning download crawl");
                crate::services::crawl::spawn(pool, c.id, Arc::clone(&storage));
            }
            Err(e) => {
                tracing::error!(link_id, crawl_type, "reactor: failed to create download crawl: {e}");
            }
        }
    }

    Ok(())
}

/// VALIDATE_CREDENTIALS crawl failed → restore old credential or mark link INVALID.
pub async fn on_validation_failed(
    pool: &PgPool,
    link_id: i32,
    old_credential_id: Option<i32>,
) -> Result<(), String> {
    tracing::info!(link_id, ?old_credential_id, "reactor: validation_failed");

    match old_credential_id {
        Some(old_id) => {
            link_repo::update_credential_and_status(pool, link_id, old_id, "VALID")
                .await
                .map_err(|e| e.to_string())?;
        }
        None => {
            link_repo::update_status(pool, link_id, "INVALID")
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
