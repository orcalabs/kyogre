DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'landings_%';

TRUNCATE TABLE landings CASCADE;
