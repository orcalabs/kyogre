ALTER TABLE data_hashes
RENAME TO file_hashes;

ALTER TABLE file_hashes
RENAME data_hash_id TO file_hash_id;
