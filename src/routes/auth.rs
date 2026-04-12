use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{services::auth as auth_service, AppState};

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegisterResponse {
    pub id: i32,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User created", body = RegisterResponse),
        (status = 409, description = "Email already in use"),
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Response {
    match auth_service::register(&state.db, &body.email, &body.password).await {
        Ok(user) => (
            StatusCode::CREATED,
            Json(RegisterResponse {
                id: user.id,
                email: user.email,
                created_at: user.created_at,
            }),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
    ),
    tag = "Auth"
)]
pub async fn login(State(state): State<AppState>, Json(body): Json<LoginRequest>) -> Response {
    match auth_service::login(&state.db, &state.jwt_secret, &body.email, &body.password).await {
        Ok(token) => Json(LoginResponse { token }).into_response(),
        Err(e) => e.into_response(),
    }
}
