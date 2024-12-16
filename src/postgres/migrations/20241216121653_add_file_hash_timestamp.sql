ALTER TABLE file_hashes
ADD COLUMN updated_at timestamptz NOT NULL DEFAULT NOW();
