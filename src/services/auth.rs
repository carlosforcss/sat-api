use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::repositories::user::{self, User};

pub enum AuthError {
    EmailAlreadyExists,
    InvalidCredentials,
    Internal,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, "email already in use"),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid credentials"),
            AuthError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
}

pub async fn register(pool: &PgPool, email: &str, password: &str) -> Result<User, AuthError> {
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|_| AuthError::Internal)?;

    user::create(pool, email, &password_hash)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("users_email_key") {
                    return AuthError::EmailAlreadyExists;
                }
            }
            AuthError::Internal
        })
}

pub async fn login(
    pool: &PgPool,
    jwt_secret: &str,
    email: &str,
    password: &str,
) -> Result<String, AuthError> {
    let user = user::find_by_email(pool, email)
        .await
        .map_err(|_| AuthError::Internal)?
        .ok_or(AuthError::InvalidCredentials)?;

    let valid = bcrypt::verify(password, &user.password_hash).map_err(|_| AuthError::Internal)?;
    if !valid {
        return Err(AuthError::InvalidCredentials);
    }

    let claims = Claims {
        sub: user.id.to_string(),
        exp: (Utc::now() + chrono::Duration::hours(24)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| AuthError::Internal)
}
