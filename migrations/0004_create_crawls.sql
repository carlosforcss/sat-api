CREATE TYPE crawl_type AS ENUM (
    'VALIDATE_CREDENTIALS',
    'DOWNLOAD_INVOICES',
    'DOWNLOAD_ISSUED_INVOICES',
    'DOWNLOAD_RECEIVED_INVOICES'
);

CREATE TYPE crawl_status AS ENUM ('PENDING', 'RUNNING', 'COMPLETED', 'FAILED');

CREATE TABLE crawls (
    id               SERIAL       PRIMARY KEY,
    user_id          INTEGER      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    link_id          INTEGER               REFERENCES links(id) ON DELETE SET NULL,
    crawl_type       crawl_type   NOT NULL,
    status           crawl_status NOT NULL DEFAULT 'PENDING',
    params           JSONB        NOT NULL DEFAULT '{}',
    response_message TEXT,
    started_at       TIMESTAMPTZ,
    finished_at      TIMESTAMPTZ,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX ON crawls (user_id);
CREATE INDEX ON crawls (link_id);
