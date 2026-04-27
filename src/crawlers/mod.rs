use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
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

fn parse_sat_datetime(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y",
    ] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, fmt)
            .map(|ndt| ndt.and_utc())
            .or_else(|_| {
                chrono::NaiveDate::parse_from_str(s, fmt)
                    .map(|nd| nd.and_hms_opt(0, 0, 0).unwrap().and_utc())
            })
        {
            return Some(dt);
        }
    }
    None
}

fn parse_sat_total(s: &str) -> Option<f64> {
    s.replace(',', "")
        .replace('$', "")
        .trim()
        .parse::<f64>()
        .ok()
}

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

impl DownloadEventHandler {
    async fn handle_file(&self, invoice: &satcrawler::Invoice, ext: &str, content: Vec<u8>) {
        let issued_at = match parse_sat_datetime(&invoice.issued_at) {
            Some(dt) => dt,
            None => {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "cannot parse issued_at: {}", invoice.issued_at
                );
                return;
            }
        };
        let certified_at = match parse_sat_datetime(&invoice.certified_at) {
            Some(dt) => dt,
            None => {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "cannot parse certified_at: {}", invoice.certified_at
                );
                return;
            }
        };
        let total = match parse_sat_total(&invoice.total) {
            Some(v) => v,
            None => {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "cannot parse total: {}", invoice.total
                );
                return;
            }
        };

        let db_invoice = match invoice_repo::create(
            &self.pool,
            self.user_id,
            Some(self.link_id),
            &invoice.uuid,
            &invoice.fiscal_id,
            &invoice.issuer_tax_id,
            &invoice.issuer_name,
            &invoice.receiver_tax_id,
            &invoice.receiver_name,
            issued_at,
            certified_at,
            total,
            &invoice.invoice_type.to_lowercase(),
            &invoice.invoice_status.to_lowercase(),
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

        let s3_key = crate::storage::invoice_s3_key(self.user_id, &invoice.uuid, ext);

        if let Err(e) = self.storage.upload(&s3_key, content).await {
            tracing::error!(
                crawl_id = self.crawl_id,
                uuid = %invoice.uuid,
                ext,
                "failed to upload to S3: {e}"
            );
            return;
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
                return;
            }
        };

        if let Err(e) = invoice_repo::set_file_id(&self.pool, db_invoice.id, ext, file.id).await {
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

#[async_trait]
impl InvoiceEventHandler for DownloadEventHandler {
    async fn should_download(&self, invoice: &satcrawler::Invoice) -> bool {
        match invoice_repo::find_by_uuid_and_user(&self.pool, &invoice.uuid, self.user_id).await {
            Ok(Some(inv)) => !(inv.xml_file_id.is_some() && inv.pdf_file_id.is_some()),
            _ => true,
        }
    }

    async fn on_invoice_event(&self, event: InvoiceEvent) {
        match event {
            InvoiceEvent::XmlDownloaded { invoice, content } => {
                self.handle_file(&invoice, "xml", content).await;
            }
            InvoiceEvent::PdfDownloaded { invoice, content } => {
                self.handle_file(&invoice, "pdf", content).await;
            }
            InvoiceEvent::XmlDownloadFailed { invoice, error } => {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "XML download failed: {error}"
                );
            }
            InvoiceEvent::PdfDownloadFailed { invoice, error } => {
                tracing::error!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "PDF download failed: {error}"
                );
            }
            InvoiceEvent::Skipped { invoice } => {
                tracing::info!(
                    crawl_id = self.crawl_id,
                    uuid = %invoice.uuid,
                    "invoice skipped"
                );
            }
        }
    }
}

async fn execute(pool: &PgPool, crawl_id: i32, storage: Arc<S3Storage>) -> Result<(), String> {
    let crawl = crawl_repo::find_by_id(pool, crawl_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("crawl {crawl_id} not found"))?;

    crawl_repo::set_running(pool, crawl_id)
        .await
        .map_err(|e| e.to_string())?;

    let link_id = crawl
        .link_id
        .ok_or_else(|| format!("crawl {crawl_id} has no link"))?;

    let link = link_repo::find_by_id(pool, link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("link {link_id} not found"))?;

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
                link_id,
                crawl_id,
                link.user_id,
                Arc::clone(&storage),
            )
            .await?
        }
        "DOWNLOAD_ISSUED_INVOICES" => {
            download_issued_invoices(
                &credential,
                &password,
                &crawl.params,
                pool,
                link_id,
                crawl_id,
                link.user_id,
                Arc::clone(&storage),
            )
            .await?
        }
        "DOWNLOAD_RECEIVED_INVOICES" => {
            download_received_invoices(
                &credential,
                &password,
                &crawl.params,
                pool,
                link_id,
                crawl_id,
                link.user_id,
                Arc::clone(&storage),
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
            crate::reactor::on_validation_succeeded(pool, storage, link_id, link.user_id).await?;
        } else {
            let old_credential_id = crawl
                .params
                .get("old_credential_id")
                .and_then(|v| v.as_i64())
                .map(|id| i32::try_from(id))
                .transpose()
                .map_err(|e| e.to_string())?;
            crate::reactor::on_validation_failed(pool, link_id, old_credential_id).await?;
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
