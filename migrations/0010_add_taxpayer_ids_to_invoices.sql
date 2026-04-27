ALTER TABLE invoices
    ADD COLUMN issuer_id   INTEGER REFERENCES taxpayers(id),
    ADD COLUMN receiver_id INTEGER REFERENCES taxpayers(id);

CREATE INDEX idx_invoices_issuer_id   ON invoices(issuer_id);
CREATE INDEX idx_invoices_receiver_id ON invoices(receiver_id);
