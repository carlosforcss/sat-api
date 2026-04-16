use std::sync::Arc;

use async_trait::async_trait;
use satcrawler::{
    Crawler, CrawlerConfig, CrawlerFilters, CrawlerOptions, CrawlerResponse, CrawlerType,
    Credentials as SatCredentials, InvoiceEvent, InvoiceEventHandler, LoginType,
    SharedInvoiceEventHandler,
};
use sqlx::PgPool;

use crate::repositories::{
    crawl as crawl_repo, credential as credential_repo, files as files_repo,
    invoice as invoice_repo, link as link_repo,
};
use crate::storage::S3Storage;

const FILE_EXTENSIONS: [&str; 2] = ["xml", "pdf"];
const UPLOAD_INITIAL_DELAY_SECS: u64 = 3;
const UPLOAD_MAX_RETRIES: u32 = 5;
const UPLOAD_RETRY_DELAY_SECS: u64 = 2;

pub async fn run_crawl(pool: &PgPool, crawl_id: i32, storage: Arc<S3Storage>) {
    if let Err(e) = execute(pool, crawl_id, storage).await {
        tracing::error!("crawl {crawl_id} failed: {e}");
        let _ = crawl_repo::set_finished(pool, crawl_id, "FAILED", Some(&e)).await;
    }
}

struct DownloadEventHandler {
    pool: PgPool,
    link_id: i32,
    crawl_id: i32,
    user_id: i32,
    storage: Arc<S3Storage>,
}

#[async_trait]
impl InvoiceEventHandler for DownloadEventHandler {
    async fn on_invoice_event(&self, event: InvoiceEvent) {
        let (invoice, download_path) = match event {
            InvoiceEvent::Downloaded {
                invoice,
                download_path,
            } => (invoice, download_path),
            InvoiceEvent::Skipped {
                invoice,
                download_path,
            } => {
                tracing::info!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    path = %download_path,
                    "invoice skipped by crawler (checking S3 status)"
                );
                (invoice, download_path)
            }
        };

        let db_invoice = match invoice_repo::create(
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
        .await
        {
            Ok(inv) => inv,
            Err(e) => {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "failed to save invoice: {e}"
                );
                return;
            }
        };

        tracing::info!(
            crawl_id = self.crawl_id,
            invoice_id = db_invoice.id,
            uuid = %db_invoice.uuid,
            "invoice metadata saved"
        );

        let fiscal_period = fiscal_period_from_issued_at(&invoice.issued_at);
        let current_period = chrono::Utc::now().format("%Y-%m").to_string();

        // Wait once before reading any files — the browser may still be writing them.
        tokio::time::sleep(tokio::time::Duration::from_secs(UPLOAD_INITIAL_DELAY_SECS)).await;

        for ext in FILE_EXTENSIONS {
            let existing_file_id = if ext == "xml" {
                db_invoice.xml_file_id
            } else {
                db_invoice.pdf_file_id
            };

            if let Some(_) = existing_file_id {
                // File was previously uploaded. Skip unless it's from the current fiscal period
                // (current-month invoices can be re-issued/updated by SAT).
                let should_skip = match &fiscal_period {
                    Some(fp) => fp != &current_period,
                    None => false, // unknown period → always re-upload to be safe
                };
                if should_skip {
                    tracing::info!(
                        crawl_id = self.crawl_id,
                        uuid = %invoice.uuid,
                        ext,
                        "skipping upload — prior fiscal period, file already stored"
                    );
                    continue;
                }
            }

            let local_path =
                std::path::Path::new(&download_path).join(format!("{}.{}", invoice.uuid, ext));

            let bytes = match read_with_retry(&local_path, UPLOAD_MAX_RETRIES).await {
                Some(b) => b,
                None => {
                    tracing::error!(
                        crawl_id = self.crawl_id,
                        uuid = %invoice.uuid,
                        ext,
                        "file not readable after retries — leaving file_id as null"
                    );
                    continue;
                }
            };

            let s3_key = crate::storage::invoice_s3_key(self.user_id, &invoice.uuid, ext);

            if let Err(e) = self.storage.upload(&s3_key, bytes).await {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    ext,
                    "failed to upload to S3: {e}"
                );
                continue;
            }

            let file = match files_repo::create(&self.pool, self.user_id, &s3_key, ext).await {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!(
                        crawl_id = self.crawl_id,
                        uuid = %invoice.uuid,
                        ext,
                        "failed to save file record: {e}"
                    );
                    continue;
                }
            };

            if let Err(e) = invoice_repo::set_file_id(&self.pool, db_invoice.id, ext, file.id).await
            {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    invoice_id = db_invoice.id,
                    ext,
                    "failed to set file_id on invoice: {e}"
                );
            } else {
                tracing::info!(
                    crawl_id = self.crawl_id,
                    invoice_id = db_invoice.id,
                    uuid = %invoice.uuid,
                    ext,
                    s3_key,
                    "invoice file uploaded to S3"
                );
            }
        }
    }
}

/// Parses `issued_at` into a "YYYY-MM" fiscal period string.
/// Tries common SAT date formats. Returns None if unparseable.
fn fiscal_period_from_issued_at(issued_at: &str) -> Option<String> {
    // Try ISO 8601 / RFC 3339 (e.g. "2026-01-15T00:00:00")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(issued_at) {
        return Some(dt.format("%Y-%m").to_string());
    }
    // Try "YYYY-MM-DD" prefix
    if issued_at.len() >= 7 && issued_at.as_bytes()[4] == b'-' {
        return Some(issued_at[..7].to_string());
    }
    // Try "DD/MM/YYYY"
    let parts: Vec<&str> = issued_at.splitn(3, '/').collect();
    if parts.len() == 3 && parts[2].len() >= 4 {
        return Some(format!("{}-{:0>2}", &parts[2][..4], parts[1]));
    }
    None
}

/// Attempts to read a file, retrying up to `max_retries` times with a fixed delay.
async fn read_with_retry(path: &std::path::Path, max_retries: u32) -> Option<Vec<u8>> {
    for attempt in 0..max_retries {
        match tokio::fs::read(path).await {
            Ok(bytes) => return Some(bytes),
            Err(_) => {
                if attempt + 1 < max_retries {
                    tokio::time::sleep(tokio::time::Duration::from_secs(UPLOAD_RETRY_DELAY_SECS))
                        .await;
                }
            }
        }
    }
    None
}

async fn execute(pool: &PgPool, crawl_id: i32, storage: Arc<S3Storage>) -> Result<(), String> {
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
                link.user_id,
                storage,
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
                link.user_id,
                storage,
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
                link.user_id,
                storage,
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

fn event_handler(
    pool: &PgPool,
    link_id: i32,
    crawl_id: i32,
    user_id: i32,
    storage: Arc<S3Storage>,
) -> SharedInvoiceEventHandler {
    Arc::new(DownloadEventHandler {
        pool: pool.clone(),
        link_id,
        crawl_id,
        user_id,
        storage,
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
    user_id: i32,
    storage: Arc<S3Storage>,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::DownloadInvoices, config)
        .with_filters(Some(filters))
        .with_event_handler(event_handler(pool, link_id, crawl_id, user_id, storage))
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
    user_id: i32,
    storage: Arc<S3Storage>,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::DownloadIssuedInvoices, config)
        .with_filters(Some(filters))
        .with_event_handler(event_handler(pool, link_id, crawl_id, user_id, storage))
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
    user_id: i32,
    storage: Arc<S3Storage>,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password);
    Ok(Crawler::new(CrawlerType::DownloadReceivedInvoices, config)
        .with_filters(Some(filters))
        .with_event_handler(event_handler(pool, link_id, crawl_id, user_id, storage))
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
