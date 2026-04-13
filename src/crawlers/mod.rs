use std::sync::Arc;

use async_trait::async_trait;
use satcrawler::{
    Crawler, CrawlerConfig, CrawlerFilters, CrawlerOptions, CrawlerResponse, CrawlerType,
    Credentials as SatCredentials, InvoiceEvent, InvoiceEventHandler, LoginType,
    SharedInvoiceEventHandler,
};
use sqlx::PgPool;

use crate::repositories::{
    crawl as crawl_repo, credential as credential_repo, invoice as invoice_repo, link as link_repo,
};

pub async fn run_crawl(pool: &PgPool, crawl_id: i32) {
    if let Err(e) = execute(pool, crawl_id).await {
        tracing::error!("crawl {crawl_id} failed: {e}");
        let _ = crawl_repo::set_finished(pool, crawl_id, "FAILED", Some(&e)).await;
    }
}

struct DownloadEventHandler {
    pool: PgPool,
    link_id: i32,
    crawl_id: i32,
}

#[async_trait]
impl InvoiceEventHandler for DownloadEventHandler {
    async fn on_invoice_event(&self, event: InvoiceEvent) {
        match event {
            InvoiceEvent::Downloaded {
                invoice,
                download_path,
            } => {
                let result = invoice_repo::create(
                    &self.pool,
                    self.link_id,
                    &invoice.uuid,
                    &invoice.fiscal_id,
                    &invoice.issuer_tax_id,
                    &invoice.issuer_name,
                    &invoice.receiver_tax_id,
                    &invoice.receiver_name,
                    &invoice.issued_at,
                    &invoice.certified_at,
                    &invoice.total,
                    &invoice.invoice_type,
                    &invoice.invoice_status,
                    &download_path,
                )
                .await;

                match result {
                    Ok(inv) => tracing::info!(
                        crawl_id = self.crawl_id,
                        invoice_id = inv.id,
                        uuid = %inv.uuid,
                        "invoice saved"
                    ),
                    Err(e) => tracing::error!(
                        crawl_id = self.crawl_id,
                        uuid = %invoice.uuid,
                        "failed to save invoice: {e}"
                    ),
                }
            }
            InvoiceEvent::Skipped {
                invoice,
                download_path,
            } => {
                tracing::info!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    path = %download_path,
                    "invoice skipped (already exists)"
                );
            }
        }
    }
}

async fn execute(pool: &PgPool, crawl_id: i32) -> Result<(), String> {
    let crawl = crawl_repo::find_by_id(pool, crawl_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("crawl {crawl_id} not found"))?;

    crawl_repo::set_running(pool, crawl_id)
        .await
        .map_err(|e| e.to_string())?;

    let link = link_repo::find_by_id(pool, crawl.link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("link {} not found", crawl.link_id))?;

    let credential = credential_repo::find_by_id(pool, link.credential_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("credential {} not found", link.credential_id))?;

    let password = crate::crypto::decrypt(&credential.password).map_err(|e| e.to_string())?;

    let response = match crawl.crawl_type.as_str() {
        "VALIDATE_CREDENTIALS" => validate_credentials(&credential, &password).await?,
        "DOWNLOAD_INVOICES" => {
            download_invoices(
                &credential,
                &password,
                &crawl.params,
                pool,
                crawl.link_id,
                crawl_id,
            )
            .await?
        }
        "DOWNLOAD_ISSUED_INVOICES" => {
            download_issued_invoices(
                &credential,
                &password,
                &crawl.params,
                pool,
                crawl.link_id,
                crawl_id,
            )
            .await?
        }
        "DOWNLOAD_RECEIVED_INVOICES" => {
            download_received_invoices(
                &credential,
                &password,
                &crawl.params,
                pool,
                crawl.link_id,
                crawl_id,
            )
            .await?
        }
        other => return Err(format!("unknown crawl type: {other}")),
    };

    let status = if response.success {
        "COMPLETED"
    } else {
        "FAILED"
    };
    crawl_repo::set_finished(pool, crawl_id, status, Some(&response.message))
        .await
        .map_err(|e| e.to_string())?;

    if crawl.crawl_type == "VALIDATE_CREDENTIALS" {
        if response.success {
            link_repo::update_status(pool, crawl.link_id, "VALID")
                .await
                .map_err(|e| e.to_string())?;
        } else {
            match crawl
                .params
                .get("old_credential_id")
                .and_then(|v| v.as_i64())
            {
                Some(old_id) => {
                    let old_credential_id = i32::try_from(old_id).map_err(|e| e.to_string())?;
                    link_repo::update_credential_and_status(
                        pool,
                        crawl.link_id,
                        old_credential_id,
                        "VALID",
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                }
                None => {
                    link_repo::update_status(pool, crawl.link_id, "INVALID")
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(())
}

fn build_config(
    credential: &crate::repositories::credential::Credential,
    password: &str,
) -> CrawlerConfig {
    let login_type = if credential.cred_type == "FIEL" {
        LoginType::Fiel
    } else {
        LoginType::Ciec
    };

    CrawlerConfig {
        credentials: SatCredentials {
            login_type,
            username: credential.taxpayer_id.clone(),
            password: password.to_string(),
            crt_path: credential.cer_path.clone(),
            key_path: credential.key_path.clone(),
        },
        options: CrawlerOptions {
            headless: true,
            sandbox: false,
        },
    }
}

fn event_handler(pool: &PgPool, link_id: i32, crawl_id: i32) -> SharedInvoiceEventHandler {
    Arc::new(DownloadEventHandler {
        pool: pool.clone(),
        link_id,
        crawl_id,
    })
}

async fn validate_credentials(
    credential: &crate::repositories::credential::Credential,
    password: &str,
) -> Result<CrawlerResponse, String> {
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::ValidateCredentials, config)
        .run()
        .await)
}

async fn download_invoices(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    params: &serde_json::Value,
    pool: &PgPool,
    link_id: i32,
    crawl_id: i32,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::DownloadInvoices, config)
        .with_filters(Some(filters))
        .with_event_handler(event_handler(pool, link_id, crawl_id))
        .run()
        .await)
}

async fn download_issued_invoices(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    params: &serde_json::Value,
    pool: &PgPool,
    link_id: i32,
    crawl_id: i32,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::DownloadIssuedInvoices, config)
        .with_filters(Some(filters))
        .with_event_handler(event_handler(pool, link_id, crawl_id))
        .run()
        .await)
}

async fn download_received_invoices(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    params: &serde_json::Value,
    pool: &PgPool,
    link_id: i32,
    crawl_id: i32,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::DownloadReceivedInvoices, config)
        .with_filters(Some(filters))
        .with_event_handler(event_handler(pool, link_id, crawl_id))
        .run()
        .await)
}

fn parse_date_filters(params: &serde_json::Value) -> Result<CrawlerFilters, String> {
    let start_date = match params.get("start_date").and_then(|v| v.as_str()) {
        Some(s) => Some(satcrawler::parse_date(s).map_err(|e| format!("invalid start_date: {e}"))?),
        None => Some(satcrawler::parse_date("01/01/2026").expect("default start_date is valid")),
    };

    let end_date = params
        .get("end_date")
        .and_then(|v| v.as_str())
        .map(|s| satcrawler::parse_date(s).map_err(|e| format!("invalid end_date: {e}")))
        .transpose()?;

    Ok(CrawlerFilters {
        start_date,
        end_date,
    })
}
