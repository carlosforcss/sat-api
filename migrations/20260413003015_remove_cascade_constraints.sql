-- Credentials deleted by users should not remove links (links represent access, not credentials)
ALTER TABLE links DROP CONSTRAINT links_credential_id_fkey;
ALTER TABLE links ADD CONSTRAINT links_credential_id_fkey
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE RESTRICT;

-- Links deleted should not remove invoices (invoices are tax documents, not link artifacts)
ALTER TABLE invoices DROP CONSTRAINT invoices_link_id_fkey;
ALTER TABLE invoices ADD CONSTRAINT invoices_link_id_fkey
    FOREIGN KEY (link_id) REFERENCES links(id) ON DELETE RESTRICT;

-- Links deleted should not remove crawl history
ALTER TABLE crawls DROP CONSTRAINT crawls_link_id_fkey;
ALTER TABLE crawls ADD CONSTRAINT crawls_link_id_fkey
    FOREIGN KEY (link_id) REFERENCES links(id) ON DELETE RESTRICT;
