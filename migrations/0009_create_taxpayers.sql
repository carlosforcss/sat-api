CREATE TABLE taxpayers (
    id              SERIAL PRIMARY KEY,
    user_id         INTEGER NOT NULL REFERENCES users(id),
    taxpayer_id     TEXT NOT NULL,
    name            TEXT NOT NULL,
    cfdi_use        TEXT,
    fiscal_domicile TEXT,
    fiscal_regime   TEXT,
    foreign_tax_id  TEXT,
    tax_residence   TEXT,
    last_seen_at    TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, taxpayer_id)
);
