DELETE FROM vessel_gear_catch_aggregates;

DELETE FROM vessel_yearly_groupings;

DELETE FROM vessel_grouping_length_combinations;

DELETE FROM vessel_groupings;

DELETE FROM orca_gear_groups;

DELETE FROM engine_transitions;

INSERT INTO
    orca_gear_groups (orca_gear_group_id, description)
VALUES
    (1, 'Trål (reke)'),
    (2, 'Trål (pelagisk)'),
    (3, 'Trål (hvitfisk)'),
    (4, 'Teine'),
    (5, 'Snurrevad'),
    (6, 'Not (pelagisk)'),
    (7, 'Not (hvitfisk)'),
    (8, 'Garn'),
    (9, 'Line'),
    (10, 'Annet');

INSERT INTO
    vessel_groupings (vessel_grouping_id, description)
VALUES
    (1, 'Trål (reke)'),
    (2, 'Trål (pelagisk)'),
    (3, 'Trål (hvitfisk)'),
    (4, 'Trål (hvitfisk/reke)'),
    (5, 'Trål/not (pelagisk)'),
    (6, 'Not (pelagisk)'),
    (7, 'Not (hvitfisk)'),
    (8, 'Teine'),
    (9, 'Snurrevad'),
    (10, 'Line og garn'),
    (11, 'Konvensjonelle'),
    (12, 'Annet');

CREATE INDEX ON vessel_yearly_groupings (YEAR, vessel_grouping_length_combination_id);

CREATE INDEX ON trips_detailed (fiskeridir_vessel_id, stop_timestamp);

INSERT INTO
    vessel_grouping_length_combinations (vessel_grouping_id, orca_vessel_length_group_id)
SELECT
    g.vessel_grouping_id,
    v.orca_vessel_length_group_id
FROM
    orca_vessel_length_groups AS v
    INNER JOIN vessel_groupings g ON TRUE;

CREATE OR REPLACE FUNCTION convert_to_orca_gear (
    species_main_group_id INT,
    gear_group_id INT,
    gear_id INT
) RETURNS INT LANGUAGE PLPGSQL AS $$
BEGIN
    RETURN CASE
        WHEN gear_group_id = 5
        AND species_main_group_id = 1 THEN 2
        WHEN gear_group_id = 5
        AND species_main_group_id = 5 THEN 1
        WHEN gear_group_id = 5 THEN 3
        WHEN gear_id = 32
        OR gear_id = 35
        OR gear_id = 31 THEN 9
        WHEN gear_group_id = 4 THEN 4
        WHEN gear_group_id = 1 AND species_main_group_id = 1 THEN 6
        WHEN gear_group_id = 1 AND species_main_group_id != 1 THEN 7
        WHEN gear_group_id = 2 THEN 8
        WHEN gear_group_id = 6 THEN 5
        ELSE 9
    END;
END
$$;

INSERT INTO
    vessel_gear_catch_aggregates (
        fiskeridir_vessel_id,
        orca_gear_group_id,
        total_living_weight,
        "year"
    )
SELECT
    fiskeridir_vessel_id,
    CONVERT_TO_ORCA_GEAR (species_main_group_id, gear_group_id, gear_id),
    SUM(living_weight),
    data_year
FROM
    landing_entries le
    INNER JOIN landings l ON le.landing_id = l.landing_id
WHERE
    living_weight IS NOT NULL
    AND fiskeridir_vessel_id IS NOT NULL
GROUP BY
    fiskeridir_vessel_id,
    data_year,
    CONVERT_TO_ORCA_GEAR (species_main_group_id, gear_group_id, gear_id);
