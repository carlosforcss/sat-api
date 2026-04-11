CREATE TYPE credential_type AS ENUM ('CIEC', 'FIEL');

CREATE TABLE credentials (
    id            SERIAL PRIMARY KEY,
    user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rfc           VARCHAR(13) NOT NULL,
    cred_type     credential_type NOT NULL,
    password_hash TEXT NOT NULL,
    cer_path      TEXT,
    key_path      TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, rfc, cred_type)
);
