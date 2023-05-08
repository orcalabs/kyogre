ALTER TABLE fiskeridir_vessels
ADD COLUMN preferred_trip_assembler INT NOT NULL DEFAULT (1) REFERENCES trip_assemblers (trip_assembler_id);

UPDATE fiskeridir_vessels
SET
    preferred_trip_assembler = 2
WHERE
    fiskeridir_vessel_id IN (
        SELECT DISTINCT
            fiskeridir_vessel_id
        FROM
            ers_departures
    );
