pub mod auth;
pub mod crawl;
pub mod credential;
pub mod health;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/credentials/ciec", post(credential::create_ciec))
        .route("/credentials/fiel", post(credential::create_fiel))
        .route("/credentials", get(credential::list_credentials))
        .route(
            "/credentials/{id}",
            axum::routing::delete(credential::delete_credential),
        )
        .route("/crawls", get(crawl::list_crawls).post(crawl::create_crawl))
        .route("/crawls/{id}", get(crawl::get_crawl))
}
