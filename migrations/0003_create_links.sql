CREATE TYPE link_status AS ENUM ('VALID', 'INVALID');

CREATE TABLE links (
    id            SERIAL      PRIMARY KEY,
    user_id       INTEGER     NOT NULL REFERENCES users(id)       ON DELETE CASCADE,
    credential_id INTEGER     NOT NULL REFERENCES credentials(id) ON DELETE RESTRICT,
    taxpayer_id   VARCHAR(13) NOT NULL,
    status        link_status NOT NULL DEFAULT 'INVALID',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, taxpayer_id)
);
