CREATE TABLE invoice_payment_complements (
    id                              SERIAL PRIMARY KEY,
    invoice_id                      INT NOT NULL UNIQUE REFERENCES invoices(id) ON DELETE CASCADE,
    version                         TEXT NOT NULL,
    total_payments_amount           NUMERIC,
    total_iva_withheld              NUMERIC,
    total_isr_withheld              NUMERIC,
    total_ieps_withheld             NUMERIC,
    total_transferred_iva_base_16   NUMERIC,
    total_transferred_iva_tax_16    NUMERIC,
    total_transferred_iva_base_8    NUMERIC,
    total_transferred_iva_tax_8     NUMERIC,
    total_transferred_iva_base_0    NUMERIC,
    total_transferred_iva_tax_0     NUMERIC,
    total_transferred_iva_base_exempt NUMERIC,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE invoice_payments (
    id                                SERIAL PRIMARY KEY,
    complement_id                     INT NOT NULL REFERENCES invoice_payment_complements(id) ON DELETE CASCADE,
    invoice_id                        INT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    payment_date                      TIMESTAMPTZ NOT NULL,
    payment_form                      TEXT NOT NULL,
    currency                          TEXT NOT NULL,
    exchange_rate                     NUMERIC,
    amount                            NUMERIC NOT NULL,
    operation_number                  TEXT,
    ordering_account_issuer_tax_id    TEXT,
    bank_name                         TEXT,
    ordering_account                  TEXT,
    beneficiary_account_issuer_tax_id TEXT,
    beneficiary_account               TEXT,
    total_transferred_tax             NUMERIC NOT NULL DEFAULT 0,
    total_withheld_tax                NUMERIC NOT NULL DEFAULT 0,
    created_at                        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoice_payments_complement_id ON invoice_payments(complement_id);
CREATE INDEX idx_invoice_payments_invoice_id    ON invoice_payments(invoice_id);
CREATE INDEX idx_invoice_payments_payment_date  ON invoice_payments(payment_date);

CREATE TABLE invoice_payment_related_documents (
    id                   SERIAL PRIMARY KEY,
    payment_id           INT NOT NULL REFERENCES invoice_payments(id) ON DELETE CASCADE,
    document_id          UUID NOT NULL,
    related_invoice_id   INT REFERENCES invoices(id) ON DELETE SET NULL,
    series               TEXT,
    fiscal_id            TEXT,
    document_currency    TEXT NOT NULL,
    exchange_equivalence NUMERIC,
    installment_number   INT NOT NULL,
    previous_balance     NUMERIC NOT NULL,
    paid_amount          NUMERIC NOT NULL,
    outstanding_balance  NUMERIC NOT NULL,
    tax_object           TEXT NOT NULL,
    total_transferred_tax NUMERIC NOT NULL DEFAULT 0,
    total_withheld_tax   NUMERIC NOT NULL DEFAULT 0,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_iprd_payment_id   ON invoice_payment_related_documents(payment_id);
CREATE INDEX idx_iprd_document_id  ON invoice_payment_related_documents(document_id);

CREATE TABLE invoice_payment_document_taxes (
    id                  SERIAL PRIMARY KEY,
    related_document_id INT NOT NULL REFERENCES invoice_payment_related_documents(id) ON DELETE CASCADE,
    tax_type            TEXT NOT NULL,
    tax                 TEXT NOT NULL,
    factor_type         TEXT,
    base                NUMERIC,
    rate_or_amount      NUMERIC,
    amount              NUMERIC,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ipdt_related_document_id ON invoice_payment_document_taxes(related_document_id);
