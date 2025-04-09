-- Add migration script here
CREATE TABLE
    IF NOT EXISTS update_checker (
        md5_checksum TEXT NOT NULL,
        last_updated TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
    )