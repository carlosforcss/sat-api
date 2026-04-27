use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    extractors::AuthUser, repositories::crawl::Crawl, services::crawl as crawl_service, AppState,
};

#[derive(Serialize, ToSchema)]
pub struct CrawlResponse {
    pub id: i32,
    pub link_id: Option<i32>,
    pub crawl_type: String,
    pub status: String,
    pub params: serde_json::Value,
    pub response_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Crawl> for CrawlResponse {
    fn from(c: Crawl) -> Self {
        CrawlResponse {
            id: c.id,
            link_id: c.link_id,
            crawl_type: c.crawl_type,
            status: c.status,
            params: c.params,
            response_message: c.response_message,
            started_at: c.started_at,
            finished_at: c.finished_at,
            created_at: c.created_at,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCrawlRequest {
    pub link_id: i32,
    pub crawl_type: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct CrawlPage {
    pub data: Vec<CrawlResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Deserialize, IntoParams)]
pub struct CrawlQueryParams {
    pub link_id: Option<i32>,
    pub crawl_type: Option<String>,
    pub status: Option<String>,
    #[serde(default = "crate::routes::default_page")]
    pub page: i64,
    #[serde(default = "crate::routes::default_per_page")]
    pub per_page: i64,
}

#[utoipa::path(
    get,
    path = "/api/crawls",
    params(CrawlQueryParams),
    responses(
        (status = 200, description = "Paginated list of crawls", body = CrawlPage),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawls"
)]
pub async fn list_crawls(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<CrawlQueryParams>,
) -> Response {
    let filters = crawl_service::CrawlFilters {
        link_id: params.link_id,
        crawl_type: params.crawl_type,
        status: params.status,
    };

    match crawl_service::list(
        &state.db,
        auth.user_id,
        filters,
        params.page,
        params.per_page,
    )
    .await
    {
        Ok((crawls, total)) => Json(CrawlPage {
            data: crawls.into_iter().map(CrawlResponse::from).collect(),
            total,
            page: params.page,
            per_page: params.per_page,
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/crawls/{id}",
    params(("id" = i32, Path, description = "Crawl ID")),
    responses(
        (status = 200, description = "Crawl detail", body = CrawlResponse),
        (status = 404, description = "Crawl not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawls"
)]
pub async fn get_crawl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Response {
    match crawl_service::get(&state.db, id, auth.user_id).await {
        Ok(c) => Json(CrawlResponse::from(c)).into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/crawls",
    request_body = CreateCrawlRequest,
    responses(
        (status = 202, description = "Crawl created and started", body = CrawlResponse),
        (status = 404, description = "Link not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Crawls"
)]
pub async fn create_crawl(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateCrawlRequest>,
) -> Response {
    let params = body.params.unwrap_or(serde_json::json!({}));
    match crawl_service::create(
        &state.db,
        state.storage.clone(),
        auth.user_id,
        body.link_id,
        &body.crawl_type,
        params,
    )
    .await
    {
        Ok(c) => (StatusCode::ACCEPTED, Json(CrawlResponse::from(c))).into_response(),
        Err(e) => e.into_response(),
    }
}
