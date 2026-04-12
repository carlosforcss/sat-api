CREATE TYPE crawl_type AS ENUM (
    'VALIDATE_CREDENTIALS',
    'DOWNLOAD_INVOICES',
    'DOWNLOAD_ISSUED_INVOICES',
    'DOWNLOAD_RECEIVED_INVOICES'
);

CREATE TYPE crawl_status AS ENUM ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED');

CREATE TABLE crawls (
    id               SERIAL PRIMARY KEY,
    credential_id    INTEGER NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    crawl_type       crawl_type NOT NULL,
    status           crawl_status NOT NULL DEFAULT 'PENDING',
    params           JSONB NOT NULL DEFAULT '{}',
    response_message TEXT,
    started_at       TIMESTAMPTZ,
    finished_at      TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
