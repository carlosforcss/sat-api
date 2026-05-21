use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

/// Unified application error. The `Display` form is the public message returned
/// to the client; routes never read variants directly so messages stay consistent.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("internal error")]
    Internal,

    #[error("{0}")]
    NotFound(&'static str),

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("email already in use")]
    EmailAlreadyExists,

    #[error("{0}")]
    Conflict(&'static str),

    #[error("{0}")]
    Unprocessable(String),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            ApiError::EmailAlreadyExists => StatusCode::CONFLICT,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}
