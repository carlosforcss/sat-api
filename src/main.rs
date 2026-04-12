use axum::{response::Html, routing::get, Json, Router};
use sqlx::postgres::PgPoolOptions;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

mod crawlers;
mod crypto;
mod extractors;
mod repositories;
mod routes;
mod services;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_secret: String,
    pub upload_path: String,
}

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&BearerAuth),
    paths(
        routes::health::health_check,
        routes::auth::register,
        routes::auth::login,
        routes::credential::create_ciec,
        routes::credential::create_fiel,
        routes::credential::list_credentials,
        routes::credential::delete_credential,
        routes::crawl::list_crawls,
        routes::crawl::get_crawl,
        routes::crawl::create_crawl,
        routes::link::create_link,
        routes::link::list_links,
        routes::link::delete_link,
    ),
    components(schemas(
        routes::auth::RegisterRequest,
        routes::auth::RegisterResponse,
        routes::auth::LoginRequest,
        routes::auth::LoginResponse,
        routes::health::HealthResponse,
        routes::credential::CreateCiecRequest,
        routes::credential::CredentialResponse,
        routes::crawl::CrawlResponse,
        routes::crawl::CreateCrawlRequest,
        routes::link::CreateLinkRequest,
        routes::link::LinkResponse,
    )),
    info(
        title = "SAT API",
        description = "Web crawling API",
        version = "0.1.0"
    )
)]
struct ApiDoc;

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn swagger_ui() -> Html<&'static str> {
    Html(
        r##"<!DOCTYPE html>
<html>
<head>
  <title>SAT API</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
<div id="swagger-ui"></div>
<script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
<script>
  window.onload = () => {
    SwaggerUIBundle({
      url: "/api/docs/openapi.json",
      dom_id: "#swagger-ui",
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: "BaseLayout"
    });

    // Swagger UI dynamically adds readonly to inputs, blocking paste.
    // Watch for those attributes and remove them immediately.
    const observer = new MutationObserver(() => {
      document.querySelectorAll('input[readonly]').forEach(el => el.removeAttribute('readonly'));
    });
    observer.observe(document.body, { subtree: true, attributes: true, attributeFilter: ['readonly'] });
  };
</script>
</body>
</html>"##,
    )
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let upload_path = std::env::var("UPLOAD_PATH").expect("UPLOAD_PATH must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let state = AppState {
        db: pool,
        jwt_secret,
        upload_path,
    };

    let app = Router::new()
        .route("/api/docs", get(swagger_ui))
        .route("/api/docs/openapi.json", get(openapi_json))
        .nest("/api", routes::router())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    tracing::info!("Server listening on http://0.0.0.0:8000");
    tracing::info!("Swagger UI at http://0.0.0.0:8000/api/docs");

    axum::serve(listener, app).await.unwrap();
}
