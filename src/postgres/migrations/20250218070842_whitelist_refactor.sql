ALTER TABLE fiskeridir_ais_vessel_active_conflicts
RENAME TO vessel_conflicts;

ALTER TABLE fiskeridir_ais_vessel_mapping_whitelist
RENAME TO all_vessels;

ALTER TABLE all_vessels
ADD COLUMN is_active BOOLEAN,
ADD COLUMN length DOUBLE PRECISION,
ADD COLUMN ship_type INT;

UPDATE all_vessels
SET
    is_active = TRUE;

UPDATE all_vessels
SET
    is_active = FALSE
WHERE
    fiskeridir_vessel_id = ANY (
        '{1999003650,
        2008044028,
        2008043350,
        2023124415,
        2001015781,
        1995002269,
        2010051209,
        2003022205}'
    );

UPDATE all_vessels v
SET
    length = q.length,
    ship_type = q.ship_type
FROM
    (
        SELECT
            v.fiskeridir_vessel_id,
            a.ship_type,
            COALESCE(f.length, a.ship_length) AS length
        FROM
            all_vessels v
            INNER JOIN fiskeridir_vessels f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
            LEFT JOIN ais_vessels a ON v.mmsi = a.mmsi
    ) q
WHERE
    v.fiskeridir_vessel_id = q.fiskeridir_vessel_id;

ALTER TABLE all_vessels
ALTER COLUMN is_active
SET NOT NULL;

CREATE UNIQUE index call_signs ON all_vessels (call_sign)
WHERE
    is_active;

CREATE UNIQUE index mmsis ON all_vessels (mmsi)
WHERE
    is_active;

CREATE VIEW active_vessels AS
SELECT
    fiskeridir_vessel_id,
    call_sign,
    mmsi,
    ship_type,
    length
FROM
    all_vessels
WHERE
    is_active;
