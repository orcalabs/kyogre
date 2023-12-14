UPDATE trips
SET
    target_species_fiskeridir_id = q.target_species_fiskeridir_id,
    target_species_fao_id = q.target_species_fao_id
FROM
    (
        SELECT
            t.trip_id,
            e.target_species_fiskeridir_id,
            e.target_species_fao_id
        FROM
            trips t
            INNER JOIN ers_departures e ON LOWER(t.period) = e.departure_timestamp
            AND t.fiskeridir_vessel_id = e.fiskeridir_vessel_id
        WHERE
            t.trip_assembler_id = 2
    ) q
WHERE
    trips.trip_id = q.trip_id
    AND trips.trip_assembler_id = 2;

UPDATE trips_detailed
SET
    target_species_fiskeridir_id = trips.target_species_fiskeridir_id,
    target_species_fao_id = trips.target_species_fao_id
FROM
    trips
WHERE
    trips_detailed.trip_id = trips.trip_id
    AND trips.trip_assembler_id = 2;
