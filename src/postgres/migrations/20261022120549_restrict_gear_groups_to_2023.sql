UPDATE fiskeridir_vessels v
SET
    gear_group_ids = q.gear_group_ids,
    species_group_ids = q.species_group_ids
FROM
    (
        SELECT
            fiskeridir_vessel_id,
            ARRAY_AGG(DISTINCT gear_group_id) AS gear_group_ids,
            ARRAY_AGG(DISTINCT species_group_id) AS species_group_ids
        FROM
            landings l
            LEFT JOIN landing_entries e ON l.landing_id = e.landing_id
        WHERE
            landing_timestamp > '2023-01-01 00:00:00'
        GROUP BY
            fiskeridir_vessel_id
    ) q
WHERE
    v.fiskeridir_vessel_id = q.fiskeridir_vessel_id;
