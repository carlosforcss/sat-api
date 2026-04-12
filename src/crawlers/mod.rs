use sqlx::PgPool;
use satcrawler::{
    Crawler, CrawlerConfig, CrawlerFilters, CrawlerOptions, CrawlerResponse, CrawlerType,
    Credentials as SatCredentials, LoginType,
};

use crate::repositories::{crawl as crawl_repo, credential as credential_repo};

pub async fn run_crawl(pool: &PgPool, crawl_id: i32) {
    if let Err(e) = execute(pool, crawl_id).await {
        tracing::error!("crawl {crawl_id} failed: {e}");
        let _ = crawl_repo::set_finished(pool, crawl_id, "FAILED", Some(&e)).await;
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

    let credential = credential_repo::find_by_id(pool, crawl.credential_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("credential {} not found", crawl.credential_id))?;

    let password = crate::crypto::decrypt(&credential.password)
        .map_err(|e| e.to_string())?;

    let response = match crawl.crawl_type.as_str() {
        "VALIDATE_CREDENTIALS" => validate_credentials(&credential, &password).await?,
        "DOWNLOAD_INVOICES" => download_invoices(&credential, &password, &crawl.params).await?,
        "DOWNLOAD_ISSUED_INVOICES" => {
            download_issued_invoices(&credential, &password, &crawl.params).await?
        }
        "DOWNLOAD_RECEIVED_INVOICES" => {
            download_received_invoices(&credential, &password, &crawl.params).await?
        }
        other => return Err(format!("unknown crawl type: {other}")),
    };

    let status = if response.success { "COMPLETED" } else { "FAILED" };
    crawl_repo::set_finished(pool, crawl_id, status, Some(&response.message))
        .await
        .map_err(|e| e.to_string())?;

    if crawl.crawl_type == "VALIDATE_CREDENTIALS" {
        let cred_status = if response.success { "VALID" } else { "UNVALID" };
        credential_repo::update_status(pool, crawl.credential_id, cred_status)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn build_config(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    filters: CrawlerFilters,
) -> CrawlerConfig {
    let login_type = if credential.cred_type == "FIEL" {
        LoginType::Fiel
    } else {
        LoginType::Ciec
    };

    CrawlerConfig {
        credentials: SatCredentials {
            login_type,
            username: credential.rfc.clone(),
            password: password.to_string(),
            crt_path: credential.cer_path.clone(),
            key_path: credential.key_path.clone(),
        },
        options: CrawlerOptions {
            headless: true,
            sandbox: false,
        },
        filters,
    }
}

async fn validate_credentials(
    credential: &crate::repositories::credential::Credential,
    password: &str,
) -> Result<CrawlerResponse, String> {
    let config = build_config(
        credential,
        password,
        CrawlerFilters {
            start_date: None,
            end_date: None,
        },
    );
    Ok(Crawler::new(CrawlerType::ValidateCredentials, config)
        .run()
        .await)
}

async fn download_invoices(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    params: &serde_json::Value,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password, filters);
    Ok(Crawler::new(CrawlerType::DownloadInvoices, config).run().await)
}

async fn download_issued_invoices(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    params: &serde_json::Value,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password, filters);
    Ok(Crawler::new(CrawlerType::DownloadIssuedInvoices, config)
        .run()
        .await)
}

async fn download_received_invoices(
    credential: &crate::repositories::credential::Credential,
    password: &str,
    params: &serde_json::Value,
) -> Result<CrawlerResponse, String> {
    let filters = parse_date_filters(params)?;
    let config = build_config(credential, password, filters);
    Ok(Crawler::new(CrawlerType::DownloadReceivedInvoices, config)
        .run()
        .await)
}

fn parse_date_filters(params: &serde_json::Value) -> Result<CrawlerFilters, String> {
    let start_date = params
        .get("start_date")
        .and_then(|v| v.as_str())
        .map(|s| {
            satcrawler::parse_date(s).map_err(|e| format!("invalid start_date: {e}"))
        })
        .transpose()?;

    let end_date = params
        .get("end_date")
        .and_then(|v| v.as_str())
        .map(|s| {
            satcrawler::parse_date(s).map_err(|e| format!("invalid end_date: {e}"))
        })
        .transpose()?;

    Ok(CrawlerFilters {
        start_date,
        end_date,
    })
}
