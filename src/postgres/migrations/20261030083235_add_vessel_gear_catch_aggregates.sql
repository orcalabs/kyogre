CREATE TABLE orca_gear_groups (
    orca_gear_group_id INT PRIMARY KEY,
    description TEXT NOT NULL CHECK (description != '')
);

DROP TABLE vessel_catch_aggregates;

DROP TABLE vessel_catch_similarity_distances;

ALTER TABLE fiskeridir_vessels
DROP COLUMN total_living_weight;

CREATE TABLE vessel_groupings (
    vessel_grouping_id INT PRIMARY KEY,
    description TEXT NOT NULL CHECK (description != '')
);

CREATE TABLE vessel_grouping_length_combinations (
    vessel_grouping_length_combination_id SERIAL PRIMARY KEY,
    vessel_grouping_id INT NOT NULL,
    orca_vessel_length_group_id INT NOT NULL REFERENCES orca_vessel_length_groups (orca_vessel_length_group_id),
    UNIQUE (vessel_grouping_id, orca_vessel_length_group_id)
);

CREATE TABLE vessel_yearly_groupings (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    "year" INT NOT NULL,
    vessel_grouping_length_combination_id INT NOT NULL REFERENCES vessel_grouping_length_combinations (vessel_grouping_length_combination_id),
    PRIMARY KEY (fiskeridir_vessel_id, "year")
);

CREATE TABLE vessel_gear_catch_aggregates (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    orca_gear_group_id INT NOT NULL REFERENCES orca_gear_groups (orca_gear_group_id),
    total_living_weight DOUBLE PRECISION NOT NULL,
    "year" INT NOT NULL,
    PRIMARY KEY (fiskeridir_vessel_id, orca_gear_group_id, "year")
);

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
        OR gear_id = 31 THEN 8
        WHEN gear_group_id = 4 THEN 4
        WHEN gear_group_id = 1 THEN 6
        WHEN gear_group_id = 2 THEN 7
        WHEN gear_group_id = 6 THEN 5
        ELSE 9
    END;
END
$$;

INSERT INTO
    vessel_groupings (vessel_grouping_id, description)
VALUES
    (1, 'Trål (reke)'),
    (2, 'Trål (pelagisk)'),
    (3, 'Trål (hvitfisk)'),
    (4, 'Trål (hvitfisk/reke)'),
    (5, 'Trål/not (pelagisk)'),
    (6, 'Not (pelagisk)'),
    (7, 'Teine'),
    (8, 'Snurrevad'),
    (9, 'Line og garn'),
    (10, 'Konvensjonelle'),
    (11, 'Annet');

INSERT INTO
    orca_gear_groups (orca_gear_group_id, description)
VALUES
    (1, 'Trål (reke)'),
    (2, 'Trål (pelagisk)'),
    (3, 'Trål (hvitfisk)'),
    (4, 'Teine'),
    (5, 'Snurrevad'),
    (6, 'Not'),
    (7, 'Garn'),
    (8, 'Line'),
    (9, 'Annet');

INSERT INTO
    vessel_grouping_length_combinations (vessel_grouping_id, orca_vessel_length_group_id)
SELECT
    g.vessel_grouping_id,
    v.orca_vessel_length_group_id
FROM
    orca_vessel_length_groups AS v
    INNER JOIN vessel_groupings g ON TRUE;

CREATE OR REPLACE FUNCTION add_to_vessel_catch_aggregates () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
  DECLARE _fiskeridir_vessel_id BIGINT;
  DECLARE _gear_id INT;
  DECLARE _gear_group_id INT;
  DECLARE _data_year INT;
  BEGIN
    IF NEW.living_weight IS NULL THEN
        RETURN NULL;
    END IF;

    SELECT INTO _fiskeridir_vessel_id, _gear_id, _gear_group_id, _data_year
        fiskeridir_vessel_id,
        gear_id,
        gear_group_id,
        data_year
    FROM
        landings
    WHERE
        landing_id = NEW.landing_id
        AND fiskeridir_vessel_id IS NOT NULL;

    IF (_fiskeridir_vessel_id IS NULL) THEN
        RETURN NULL;
    END IF;

    INSERT INTO
        vessel_gear_catch_aggregates (
            fiskeridir_vessel_id,
            orca_gear_group_id,
            total_living_weight,
            year
        )
    VALUES
        (
            _fiskeridir_vessel_id,
            CONVERT_TO_ORCA_GEAR(NEW.species_main_group_id, _gear_group_id, _gear_id),
            NEW.living_weight,
            _data_year
        )
    ON CONFLICT (fiskeridir_vessel_id, orca_gear_group_id, year) DO UPDATE
    SET
        total_living_weight = vessel_gear_catch_aggregates.total_living_weight + EXCLUDED.total_living_weight;

    RETURN NULL;
  END;
$$;

CREATE OR REPLACE FUNCTION remove_from_vessel_catch_aggregates () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
  DECLARE _fiskeridir_vessel_id BIGINT;
  BEGIN
    IF OLD.living_weight IS NULL THEN
        RETURN NULL;
    END IF;

    SELECT INTO _fiskeridir_vessel_id
        fiskeridir_vessel_id
    FROM
        landings
    WHERE
        landing_id = NEW.landing_id
        AND fiskeridir_vessel_id IS NOT NULL;

    UPDATE vessel_gear_catch_aggregates
    SET
        total_living_weight = total_living_weight - OLD.living_weight
    WHERE
        fiskeridir_vessel_id = _fiskeridir_vessel_id
        AND orca_gear_group_id = OLD.orca_gear_group_id
        AND data_year = OLD.data_year;

    RETURN NULL;
  END;
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
