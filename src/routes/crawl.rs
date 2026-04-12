use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{extractors::AuthUser, services::crawl as crawl_service, AppState};

#[derive(Serialize, ToSchema)]
pub struct CrawlResponse {
    pub id: i32,
    pub credential_id: i32,
    pub crawl_type: String,
    pub status: String,
    pub params: serde_json::Value,
    pub response_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, IntoParams)]
pub struct CrawlQueryParams {
    pub credential_id: Option<i32>,
    pub crawl_type: Option<String>,
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/crawls",
    params(CrawlQueryParams),
    responses(
        (status = 200, description = "List of crawls", body = Vec<CrawlResponse>),
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
        credential_id: params.credential_id,
        crawl_type: params.crawl_type,
        status: params.status,
    };

    match crawl_service::list(&state.db, auth.user_id, filters).await {
        Ok(crawls) => Json(
            crawls
                .into_iter()
                .map(|c| CrawlResponse {
                    id: c.id,
                    credential_id: c.credential_id,
                    crawl_type: c.crawl_type,
                    status: c.status,
                    params: c.params,
                    response_message: c.response_message,
                    started_at: c.started_at,
                    finished_at: c.finished_at,
                    created_at: c.created_at,
                })
                .collect::<Vec<_>>(),
        )
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
        Ok(c) => Json(CrawlResponse {
            id: c.id,
            credential_id: c.credential_id,
            crawl_type: c.crawl_type,
            status: c.status,
            params: c.params,
            response_message: c.response_message,
            started_at: c.started_at,
            finished_at: c.finished_at,
            created_at: c.created_at,
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
}
