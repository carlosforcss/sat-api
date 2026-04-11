CREATE TYPE credential_status AS ENUM ('VALID', 'UNVALID', 'UNKNOWN');

ALTER TABLE credentials
    ADD COLUMN status credential_status NOT NULL DEFAULT 'UNKNOWN';
