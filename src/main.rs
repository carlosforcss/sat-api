use axum::{response::Html, routing::get, Json, Router};
use sqlx::postgres::PgPoolOptions;
use utoipa::OpenApi;

mod repositories;
mod routes;
mod services;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_secret: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::health_check,
        routes::auth::register,
        routes::auth::login,
    ),
    components(schemas(
        routes::auth::RegisterRequest,
        routes::auth::RegisterResponse,
        routes::auth::LoginRequest,
        routes::auth::LoginResponse,
        routes::health::HealthResponse,
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
    };

    let app = Router::new()
        .route("/api/docs", get(swagger_ui))
        .route("/api/docs/openapi.json", get(openapi_json))
        .nest("/api", routes::router())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .unwrap();

    tracing::info!("Server listening on http://0.0.0.0:8000");
    tracing::info!("Swagger UI at http://0.0.0.0:8000/api/docs");

    axum::serve(listener, app).await.unwrap();
}
