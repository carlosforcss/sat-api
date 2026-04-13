CREATE TYPE credential_type AS ENUM ('CIEC', 'FIEL');

CREATE TYPE credential_status AS ENUM ('VALID', 'INVALID', 'UNKNOWN');

CREATE TABLE credentials (
    id          SERIAL           PRIMARY KEY,
    user_id     INTEGER          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    taxpayer_id VARCHAR(13)      NOT NULL,
    cred_type   credential_type  NOT NULL,
    status      credential_status NOT NULL DEFAULT 'UNKNOWN',
    password    TEXT             NOT NULL,
    cer_path    TEXT,
    key_path    TEXT,
    created_at  TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ      NOT NULL DEFAULT NOW()
);

CREATE INDEX ON credentials (user_id);
