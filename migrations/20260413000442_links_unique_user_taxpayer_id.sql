ALTER TABLE links ADD CONSTRAINT links_user_id_taxpayer_id_key UNIQUE (user_id, taxpayer_id);
