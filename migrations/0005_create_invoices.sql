CREATE TYPE invoice_type_enum   AS ENUM ('ingreso', 'egreso', 'traslado', 'nómina', 'pago');
CREATE TYPE invoice_status_enum AS ENUM ('vigente', 'cancelado');

CREATE TABLE invoices (
    id                   SERIAL              PRIMARY KEY,
    user_id              INTEGER             NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    link_id              INTEGER                      REFERENCES links(id)  ON DELETE SET NULL,
    uuid                 UUID                NOT NULL,
    fiscal_id            TEXT                NOT NULL,
    issuer_taxpayer_id   TEXT                NOT NULL,
    issuer_name          TEXT                NOT NULL,
    receiver_taxpayer_id TEXT                NOT NULL,
    receiver_name        TEXT                NOT NULL,
    issued_at            TIMESTAMPTZ         NOT NULL,
    certified_at         TIMESTAMPTZ         NOT NULL,
    total                NUMERIC             NOT NULL,
    invoice_type         invoice_type_enum   NOT NULL,
    invoice_status       invoice_status_enum NOT NULL,
    created_at           TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
    UNIQUE (uuid, user_id)
);

CREATE INDEX ON invoices (user_id);
CREATE INDEX ON invoices (link_id);
CREATE INDEX ON invoices (issued_at DESC);
CREATE INDEX ON invoices (issuer_taxpayer_id);
CREATE INDEX ON invoices (receiver_taxpayer_id);
