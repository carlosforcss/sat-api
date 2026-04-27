CREATE TABLE invoice_items (
    id                    SERIAL PRIMARY KEY,
    invoice_id            INT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    product_service_key   TEXT NOT NULL,
    id_number             TEXT,
    quantity              NUMERIC NOT NULL,
    unit_key              TEXT NOT NULL,
    unit                  TEXT,
    description           TEXT NOT NULL,
    unit_value            NUMERIC NOT NULL,
    amount                NUMERIC NOT NULL,
    discount              NUMERIC,
    tax_object            TEXT,
    third_party           JSONB,
    customs_info          JSONB NOT NULL DEFAULT '[]',
    property_tax_accounts JSONB NOT NULL DEFAULT '[]',
    parts                 JSONB NOT NULL DEFAULT '[]',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoice_items_invoice_id ON invoice_items(invoice_id);

CREATE TABLE invoice_item_taxes (
    id             SERIAL PRIMARY KEY,
    item_id        INT NOT NULL REFERENCES invoice_items(id) ON DELETE CASCADE,
    tax_type       TEXT NOT NULL,
    tax            TEXT NOT NULL,
    factor_type    TEXT,
    base           NUMERIC,
    rate_or_amount NUMERIC,
    amount         NUMERIC,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoice_item_taxes_item_id ON invoice_item_taxes(item_id);
