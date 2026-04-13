use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{extractors::AuthUser, services::link as link_service, AppState};

#[derive(Serialize, ToSchema)]
pub struct LinkResponse {
    pub id: i32,
    pub credential_id: i32,
    pub taxpayer_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct LinkPage {
    pub data: Vec<LinkResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Deserialize, IntoParams)]
pub struct LinkQueryParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

#[utoipa::path(
    get,
    path = "/api/links",
    params(LinkQueryParams),
    responses(
        (status = 200, description = "Paginated list of links", body = LinkPage),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Links"
)]
pub async fn list_links(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<LinkQueryParams>,
) -> Response {
    match link_service::list(&state.db, auth.user_id, params.page, params.per_page).await {
        Ok((links, total)) => Json(LinkPage {
            data: links
                .into_iter()
                .map(|lnk| LinkResponse {
                    id: lnk.id,
                    credential_id: lnk.credential_id,
                    taxpayer_id: lnk.taxpayer_id,
                    status: lnk.status,
                    created_at: lnk.created_at,
                })
                .collect(),
            total,
            page: params.page,
            per_page: params.per_page,
        })
        .into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/api/links/{id}",
    params(("id" = i32, Path, description = "Link ID")),
    responses(
        (status = 204, description = "Link deleted"),
        (status = 404, description = "Link not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Links"
)]
pub async fn delete_link(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Response {
    match link_service::delete(&state.db, id, auth.user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "link not found" })),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}
