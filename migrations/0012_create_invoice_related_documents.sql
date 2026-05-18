CREATE TABLE invoice_related_documents (
    id                 SERIAL PRIMARY KEY,
    invoice_id         INT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    relation_type      TEXT NOT NULL,
    related_uuid       UUID NOT NULL,
    related_invoice_id INT REFERENCES invoices(id) ON DELETE SET NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invoice_related_documents_invoice_id
    ON invoice_related_documents(invoice_id);
CREATE INDEX idx_invoice_related_documents_related_uuid
    ON invoice_related_documents(related_uuid);
