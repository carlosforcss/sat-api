use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub storage: String,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Database or storage unreachable"),
    ),
    tag = "Health"
)]
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<HealthResponse>)> {
    let (db_ok, s3_ok) = tokio::join!(
        async { sqlx::query("SELECT 1").execute(&state.db).await.is_ok() },
        state.storage.is_reachable(),
    );

    let response = HealthResponse {
        status: if db_ok && s3_ok { "ok" } else { "degraded" }.to_string(),
        database: if db_ok { "connected" } else { "unreachable" }.to_string(),
        storage: if s3_ok { "connected" } else { "unreachable" }.to_string(),
    };

    if db_ok && s3_ok {
        Ok(Json(response))
    } else {
        Err((StatusCode::SERVICE_UNAVAILABLE, Json(response)))
    }
}
