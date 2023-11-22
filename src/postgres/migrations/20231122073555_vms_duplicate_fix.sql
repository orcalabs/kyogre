TRUNCATE TABLE vms_positions;

DROP INDEX vms_positions_call_sign_message_id_idx;

DROP INDEX vms_positions_timestamp_idx1;

ALTER TABLE vms_positions
ADD PRIMARY KEY (call_sign, "timestamp");

DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'vms_%';
