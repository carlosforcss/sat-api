use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    extractors::AuthUser, repositories::credential::Credential,
    services::credential as credential_service, AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct CreateCiecRequest {
    pub taxpayer_id: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct CredentialResponse {
    pub id: i32,
    pub taxpayer_id: String,
    pub cred_type: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct CredentialPage {
    pub data: Vec<CredentialResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

impl From<Credential> for CredentialResponse {
    fn from(c: Credential) -> Self {
        CredentialResponse {
            id: c.id,
            taxpayer_id: c.taxpayer_id,
            cred_type: c.cred_type,
            status: c.status,
            created_at: c.created_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct CredentialQueryParams {
    #[serde(default = "crate::routes::default_page")]
    pub page: i64,
    #[serde(default = "crate::routes::default_per_page")]
    pub per_page: i64,
}

#[utoipa::path(
    post,
    path = "/api/credentials/ciec",
    request_body = CreateCiecRequest,
    responses(
        (status = 201, description = "CIEC credential created", body = CredentialResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Credentials"
)]
pub async fn create_ciec(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateCiecRequest>,
) -> Response {
    match credential_service::create_ciec(
        &state.db,
        state.storage.clone(),
        auth.user_id,
        &body.taxpayer_id,
        &body.password,
    )
    .await
    {
        Ok(cred) => (StatusCode::CREATED, Json(CredentialResponse::from(cred))).into_response(),
        Err(e) => e.into_response(),
    }
}

#[derive(ToSchema)]
#[allow(dead_code)]
pub struct CreateFielRequest {
    pub taxpayer_id: String,
    pub password: String,
    #[schema(value_type = String, format = Binary)]
    pub cer_file: Vec<u8>,
    #[schema(value_type = String, format = Binary)]
    pub key_file: Vec<u8>,
}

#[utoipa::path(
    post,
    path = "/api/credentials/fiel",
    request_body(content = inline(CreateFielRequest), content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "FIEL credential created", body = CredentialResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Missing required fields"),
    ),
    security(("bearer_auth" = [])),
    tag = "Credentials"
)]
pub async fn create_fiel(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Response {
    let mut taxpayer_id: Option<String> = None;
    let mut password: Option<String> = None;
    let mut cer_bytes: Option<Vec<u8>> = None;
    let mut key_bytes: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("taxpayer_id") => {
                taxpayer_id = field.text().await.ok();
            }
            Some("password") => {
                password = field.text().await.ok();
            }
            Some("cer_file") => {
                cer_bytes = field.bytes().await.ok().map(|b| b.to_vec());
            }
            Some("key_file") => {
                key_bytes = field.bytes().await.ok().map(|b| b.to_vec());
            }
            _ => {}
        }
    }

    let (Some(taxpayer_id), Some(password), Some(cer_bytes), Some(key_bytes)) =
        (taxpayer_id, password, cer_bytes, key_bytes)
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "missing required fields: taxpayer_id, password, cer_file, key_file" })),
        )
            .into_response();
    };

    match credential_service::create_fiel(
        &state.db,
        state.storage.clone(),
        &state.upload_path,
        auth.user_id,
        &taxpayer_id,
        &password,
        cer_bytes,
        key_bytes,
    )
    .await
    {
        Ok(cred) => (StatusCode::CREATED, Json(CredentialResponse::from(cred))).into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/credentials",
    params(CredentialQueryParams),
    responses(
        (status = 200, description = "Paginated list of credentials", body = CredentialPage),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Credentials"
)]
pub async fn list_credentials(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<CredentialQueryParams>,
) -> Response {
    match credential_service::list(&state.db, auth.user_id, params.page, params.per_page).await {
        Ok((creds, total)) => Json(CredentialPage {
            data: creds.into_iter().map(CredentialResponse::from).collect(),
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
    path = "/api/credentials/{id}",
    params(("id" = i32, Path, description = "Credential ID")),
    responses(
        (status = 204, description = "Credential deleted"),
        (status = 404, description = "Credential not found"),
        (status = 409, description = "Credential is in use by a link"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Credentials"
)]
pub async fn delete_credential(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i32>,
) -> Response {
    match credential_service::delete(&state.db, id, auth.user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "credential not found" })),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}
