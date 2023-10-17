TRUNCATE TABLE vms_positions;

DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'vms_%';

ALTER TABLE vms_positions
ADD COLUMN distance_to_shore DOUBLE PRECISION NOT NULL;
