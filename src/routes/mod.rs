pub mod health;

use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", axum::routing::get(health::health_check))
}
