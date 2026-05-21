use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::ApiError;
use crate::repositories::user::{self, User};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,
    pub exp: i64,
}

pub async fn register(pool: &PgPool, email: &str, password: &str) -> Result<User, ApiError> {
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|_| ApiError::Internal)?;

    user::create(pool, email, &password_hash)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("users_email_key") {
                    return ApiError::EmailAlreadyExists;
                }
            }
            ApiError::Internal
        })
}

pub async fn login(
    pool: &PgPool,
    jwt_secret: &str,
    email: &str,
    password: &str,
) -> Result<String, ApiError> {
    let user = user::find_by_email(pool, email)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or(ApiError::InvalidCredentials)?;

    let valid = bcrypt::verify(password, &user.password_hash).map_err(|_| ApiError::Internal)?;
    if !valid {
        return Err(ApiError::InvalidCredentials);
    }

    let claims = Claims {
        sub: user.id,
        exp: (Utc::now() + chrono::Duration::hours(24)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| ApiError::Internal)
}
