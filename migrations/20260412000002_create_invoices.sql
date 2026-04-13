CREATE TABLE invoices (
    id                   SERIAL PRIMARY KEY,
    link_id              INTEGER NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    uuid                 VARCHAR(36) NOT NULL,
    fiscal_id            TEXT NOT NULL,
    issuer_taxpayer_id   TEXT NOT NULL,
    issuer_name          TEXT NOT NULL,
    receiver_taxpayer_id TEXT NOT NULL,
    receiver_name        TEXT NOT NULL,
    issued_at            TEXT NOT NULL,
    certified_at         TEXT NOT NULL,
    total                TEXT NOT NULL,
    invoice_type         TEXT NOT NULL,
    invoice_status       TEXT NOT NULL,
    download_path        TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
