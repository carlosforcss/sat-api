# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Running the project

Everything runs through Docker Compose — do not run Postgres or the API directly on the host.

```bash
# Start all services (builds the API image on first run)
docker compose up --build

# Rebuild the API image after code changes
docker compose up --build api

# Start only Postgres (useful when running the API locally with cargo)
docker compose up postgres
```

For local development without Docker (requires Postgres running separately):

```bash
cargo run          # starts the API on :8000
cargo check        # type-check without building
cargo build        # debug build
cargo build --release
```

## Environment variables

Copy `.env.example` to `.env` before running locally. The only required variable is:

```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/satcli
RUST_LOG=info      # controls tracing output level
```

When running via Docker Compose the `api` service injects these automatically (host is `postgres`, not `localhost`).

## Database migrations

Migrations live in `migrations/` and are run automatically on startup via `sqlx::migrate!()`. To add a new migration, create a file named `migrations/YYYYMMDDHHMMSS_description.sql`. sqlx runs them in filename order and tracks applied migrations in the `_sqlx_migrations` table.

## Guiding principle

Always take the simplest approach that satisfies the requirement. Do not add abstractions, helpers, or extra layers unless they are immediately needed. Prefer flat, explicit code over clever or generic code.

## Architecture

`main.rs` is the entry point — it wires together the DB pool, runs migrations, registers routes, and starts the server. There is no framework-level config file; everything is done in code.

**State** — `AppState` (defined in `main.rs`) holds the `sqlx::PgPool` and is injected into handlers via axum's `State` extractor. It must be `Clone`.

**Layer structure** — the codebase follows a three-layer pattern:

```
src/routes/      — HTTP handlers (axum extractors, request/response shapes, utoipa annotations)
src/services/    — business logic (orchestrates repositories, owns domain rules)
src/repositories/ — database access (raw sqlx queries, one file per entity)
```

Rules:
- Routes call services. Routes never touch the DB directly.
- Services call repositories. Services never write raw SQL.
- Repositories only query the DB — no business logic.
- No aggregator layer. Domain objects are plain Rust structs; keep them in the module they belong to until there is a concrete reason to move them.

**Reactor** — `src/reactor.rs` is the single place for all domain event reactions ("when A happens, do B"). Any time a completed action should trigger a background side effect, add a function here. Current reactions:

- `on_credential_created` — called by services after credential creation; sets up the link and spawns a `VALIDATE_CREDENTIALS` crawl.
- `on_validation_succeeded` — called by the crawler after successful validation; marks link `VALID` and spawns `DOWNLOAD_ISSUED_INVOICES` + `DOWNLOAD_RECEIVED_INVOICES` crawls.
- `on_validation_failed` — called by the crawler on failure; restores the previous credential or marks the link `INVALID`.

Each reactor function emits a structured `tracing::info!` log with the prefix `reactor:` so the full event chain is visible with `RUST_LOG=info`.

**Crawl execution** — `services/crawl.rs::spawn` runs each crawl in a dedicated `std::thread` with its own tokio runtime. This isolation is required because the underlying crawler uses headless Chromium (via `chromiumoxide`), which cannot share the axum tokio runtime. A global semaphore (`MAX_CONCURRENT_CRAWLS = 3`) caps concurrent crawl threads.

**OpenAPI / Swagger** — `utoipa` generates the spec from `#[utoipa::path]` macros on each handler. `ApiDoc` in `main.rs` collects all paths. The spec is served at `GET /api/docs/openapi.json` and the Swagger UI at `GET /api/docs`.

**Router assembly** — `swagger_ui` and `openapi_json` handlers are stateless, so they are registered directly on the top-level `Router` before `.with_state(state)` is called. Stateful routes go inside `routes::router()`.

## Adding a new endpoint

1. Create `src/repositories/<entity>.rs` — sqlx queries for that entity.
2. Create `src/services/<domain>.rs` — business logic calling the repository.
3. Create `src/routes/<domain>.rs` — handler with `#[utoipa::path]`, calls the service.
4. Register the module and route in `src/routes/mod.rs`.
5. Add the handler path to `#[openapi(paths(...))]` in `main.rs`.
