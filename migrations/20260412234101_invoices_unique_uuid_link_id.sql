ALTER TABLE invoices ADD CONSTRAINT invoices_uuid_link_id_key UNIQUE (uuid, link_id);
