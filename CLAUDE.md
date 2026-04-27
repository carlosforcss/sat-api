# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Running the project

Docker Compose runs all three services (Postgres, MinIO, API). The API image uses `cargo watch -x run` for hot reload; compiled artifacts live in the `cargo_target` named volume so they survive container restarts.

```bash
# Start all services (builds the API image on first run)
docker compose up --build

# Rebuild the API image after code changes
docker compose up --build api

# Start only Postgres + MinIO (useful when running the API locally with cargo)
docker compose up postgres minio
```

For local development without Docker (requires Postgres and MinIO running separately):

```bash
cargo run          # starts the API on :8000
cargo check        # type-check without building
cargo build        # debug build
```

## Environment variables

Copy `.env.example` to `.env` before running. All variables in `.env.example` are required:

| Variable | Description |
|---|---|
| `DATABASE_URL` | Postgres connection string |
| `RUST_LOG` | Tracing level (`info,chromiumoxide=off` recommended) |
| `JWT_SECRET` | Secret for signing/verifying JWT tokens |
| `UPLOAD_PATH` | Local path for FIEL certificate uploads |
| `TWOCAPTCHA_API_KEY` | 2captcha API key (used by the SAT crawler) |
| `CREDENTIAL_ENCRYPTION_KEY` | 32-byte key for AES-GCM credential encryption |
| `S3_BUCKET` | S3/MinIO bucket name |
| `AWS_REGION` | AWS/MinIO region |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | Credentials (use `minioadmin` locally) |
| `AWS_ENDPOINT_URL` | Override endpoint for local MinIO (`http://localhost:9000`) |

When running via Docker Compose the `api` service injects these automatically (DB host is `postgres`, MinIO host is `minio`).

## Database migrations

Migrations live in `migrations/` and run automatically on startup via `sqlx::migrate!()`. Add new migrations as `migrations/NNNN_description.sql` — sqlx runs them in filename order and tracks applied migrations in `_sqlx_migrations`.

## Guiding principle

Always take the simplest approach that satisfies the requirement. Do not add abstractions, helpers, or extra layers unless they are immediately needed. Prefer flat, explicit code over clever or generic code.

## Architecture

`main.rs` is the entry point — it wires together the DB pool, runs migrations, builds `AppState`, registers routes, and starts the server.

**State** — `AppState` (in `main.rs`) holds the `sqlx::PgPool`, `jwt_secret`, `upload_path`, and an `Arc<S3Storage>`. It is injected into handlers via axum's `State` extractor and must be `Clone`.

**Layer structure**

```
src/routes/       — HTTP handlers (axum extractors, request/response shapes, utoipa annotations)
src/services/     — business logic (orchestrates repositories, owns domain rules)
src/repositories/ — database access (raw sqlx queries, one file per entity)
```

Rules:
- Routes call services. Routes never touch the DB directly.
- Services call repositories. Services never write raw SQL.
- Repositories only query the DB — no business logic.

**Shared module helpers**
- `repositories::is_fk_violation(e: &sqlx::Error) -> bool` — checks for Postgres error code 23503.
- `services::paginate(page, per_page) -> (page, per_page, offset)` — clamps `per_page` to 1–100 and `page` ≥ 1 before computing offset. Always call this before passing limit/offset to a repository.

**Storage** — `src/storage.rs` wraps the AWS S3 SDK as `S3Storage`. Invoice files are keyed as `invoices/{user_id}/{uuid}/{uuid}.{extension}` via `storage::invoice_s3_key(user_id, uuid, extension)`. Locally, MinIO provides a compatible S3 endpoint.

**Reactor** — `src/reactor.rs` is the single place for domain event reactions. Any time a completed action should trigger a background side effect, add a function here. Current reactions:

- `on_credential_created` — sets up the link and spawns a `VALIDATE_CREDENTIALS` crawl.
- `on_validation_succeeded` — marks link `VALID`, spawns `DOWNLOAD_ISSUED_INVOICES` + `DOWNLOAD_RECEIVED_INVOICES` crawls.
- `on_validation_failed` — restores the previous credential or marks the link `INVALID`.

Each reactor function emits a `tracing::info!` log with the prefix `reactor:` so the full event chain is visible at `RUST_LOG=info`.

**Crawl execution** — `services/crawl.rs::spawn` runs each crawl in a dedicated `std::thread` with its own tokio runtime. This isolation is required because the crawler uses headless Chromium (`chromiumoxide`), which cannot share the axum tokio runtime. A global semaphore caps `MAX_CONCURRENT_CRAWLS = 3`. Recognized crawl types: `VALIDATE_CREDENTIALS`, `DOWNLOAD_INVOICES`, `DOWNLOAD_ISSUED_INVOICES`, `DOWNLOAD_RECEIVED_INVOICES`.

**External crates** (both from the same private git repo)
- `satcrawler` — headless Chromium crawler that logs into the SAT portal and downloads CFDI XMLs/PDFs. Exposes `Crawler`, `CrawlerType`, `InvoiceEventHandler`, and date parsing utilities.
- `sat-cfdi` — pure Rust parser for Mexican CFDI XML invoices. Key API: `parse_bytes(&[u8]) -> Result<Invoice, CfdiError>` and `parse_cfdi_datetime(&str) -> Result<NaiveDateTime, ParseError>`. The parsed `Invoice` struct mirrors the CFDI 4.0 schema (`issuer`, `recipient`, `line_items`, `taxes`, etc.).

**Auth** — `src/extractors.rs` defines `AuthUser`, an axum extractor that validates the `Authorization: Bearer <JWT>` header and injects `user_id: i32` into handlers. All user-scoped data uses this `user_id` as a filter — there is no admin bypass.

**OpenAPI / Swagger** — `utoipa` generates the spec from `#[utoipa::path]` macros. `ApiDoc` in `main.rs` collects all paths and schemas. Spec at `GET /api/docs/openapi.json`, UI at `GET /api/docs`.

**Router assembly** — `swagger_ui` and `openapi_json` are stateless and registered on the top-level `Router` before `.with_state(state)`. Stateful routes go inside `routes::router()`. Register path-param routes (`/invoices/{id}`) after any literal-segment routes at the same prefix (`/invoices/parse-all`) to avoid axum treating the literal as an ID.

## Adding a new endpoint

1. Create `src/repositories/<entity>.rs` — sqlx queries for that entity.
2. Create `src/services/<domain>.rs` — business logic calling the repository.
3. Create `src/routes/<domain>.rs` — handler with `#[utoipa::path]`, calls the service.
4. Register the module and route in `src/routes/mod.rs`.
5. Add the handler path and any new schemas to `#[openapi(paths(...), components(schemas(...)))]` in `main.rs`.
