CREATE
OR REPLACE FUNCTION public.to_vessel_length_group (vessel_length NUMERIC) RETURNS INTEGER LANGUAGE plpgsql IMMUTABLE AS $$
    BEGIN
        RETURN
            CASE
                WHEN vessel_length IS NULL THEN 0
                WHEN vessel_length < 11 THEN 1
                WHEN vessel_length <@ numrange(11, 15, '[)') THEN 2
                WHEN vessel_length <@ numrange(15, 21, '[)') THEN 3
                WHEN vessel_length <@ numrange(21, 28, '[)') THEN 4
            ELSE 5
        END;
    END;
$$;

ALTER TABLE fiskeridir_vessels
DROP COLUMN fiskeridir_length_group_id;

ALTER TABLE fiskeridir_vessels
ADD COLUMN fiskeridir_length_group_id INT NOT NULL REFERENCES fiskeridir_length_groups (fiskeridir_length_group_id) GENERATED ALWAYS AS (TO_VESSEL_LENGTH_GROUP (LENGTH)) STORED;

UPDATE trips_detailed t
SET
    fiskeridir_length_group_id = v.fiskeridir_length_group_id
FROM
    fiskeridir_vessels v
WHERE
    t.fiskeridir_vessel_id = v.fiskeridir_vessel_id;
