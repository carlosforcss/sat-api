use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Json, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::json;

use crate::{services::auth::Claims, AppState};

pub struct AuthUser {
    pub user_id: i32,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "missing authorization header" })),
                )
                    .into_response()
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid authorization header" })),
            )
                .into_response()
        })?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid or expired token" })),
            )
                .into_response()
        })?;

        let user_id: i32 = token_data.claims.sub.parse().map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid token claims" })),
            )
                .into_response()
        })?;

        Ok(AuthUser { user_id })
    }
}
