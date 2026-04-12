use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{extractors::AuthUser, services::link as link_service, AppState};

#[derive(Deserialize, ToSchema)]
pub struct CreateLinkRequest {
    pub credential_id: i32,
}

#[derive(Serialize, ToSchema)]
pub struct LinkResponse {
    pub id: i32,
    pub credential_id: i32,
    pub taxpayer_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[utoipa::path(
    post,
    path = "/api/links",
    request_body = CreateLinkRequest,
    responses(
        (status = 201, description = "Link created", body = LinkResponse),
        (status = 404, description = "Credential not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Links"
)]
pub async fn create_link(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateLinkRequest>,
) -> Response {
    match link_service::create(&state.db, auth.user_id, body.credential_id).await {
        Ok(lnk) => (
            StatusCode::CREATED,
            Json(LinkResponse {
                id: lnk.id,
                credential_id: lnk.credential_id,
                taxpayer_id: lnk.taxpayer_id,
                status: lnk.status,
                created_at: lnk.created_at,
            }),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/links",
    responses(
        (status = 200, description = "List of links", body = Vec<LinkResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Links"
)]
pub async fn list_links(State(state): State<AppState>, auth: AuthUser) -> Response {
    match link_service::list(&state.db, auth.user_id).await {
        Ok(links) => Json(
            links
                .into_iter()
                .map(|lnk| LinkResponse {
                    id: lnk.id,
                    credential_id: lnk.credential_id,
                    taxpayer_id: lnk.taxpayer_id,
                    status: lnk.status,
                    created_at: lnk.created_at,
                })
                .collect::<Vec<_>>(),
        )
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
