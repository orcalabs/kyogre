CREATE TABLE vessel_catch_aggregates (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
    total_living_weight DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (fiskeridir_vessel_id, species_group_id)
);

ALTER TABLE fiskeridir_vessels
ADD COLUMN total_living_weight DOUBLE PRECISION NOT NULL DEFAULT 0.0;

ALTER TABLE trips_detailed
ADD COLUMN benchmark_catch_value_per_fuel_liter_estimated_only DOUBLE PRECISION,
ADD COLUMN benchmark_weight_per_fuel_liter_estimated_only DOUBLE PRECISION;

UPDATE trips_detailed
SET
    benchmark_status = 1;

CREATE TABLE vessel_catch_similarity_distances (
    vessel_one BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    vessel_two BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    distance DOUBLE PRECISION NOT NULL,
    UNIQUE (vessel_two, vessel_one),
    PRIMARY KEY (vessel_one, vessel_two)
);

CREATE INDEX ON vessel_catch_similarity_distances (vessel_one, distance);

CREATE INDEX ON vessel_catch_similarity_distances (vessel_two, distance);

CREATE OR REPLACE FUNCTION add_to_vessel_catch_aggregates () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
  DECLARE _landing_timestamp TIMESTAMPTZ;
  DECLARE _fiskeridir_vessel_id BIGINT;
  BEGIN
    IF NEW.living_weight IS NULL THEN
        RETURN NULL;
    END IF;

    SELECT
        landing_timestamp,
        fiskeridir_vessel_id INTO _landing_timestamp,
        _fiskeridir_vessel_id
    FROM
        landings
    WHERE
        landing_id = NEW.landing_id
        AND fiskeridir_vessel_id IS NOT NULL;

    IF (_landing_timestamp IS NULL OR _landing_timestamp < '2023-01-01 00:00:00') THEN
        RETURN NULL;
    END IF;

    UPDATE fiskeridir_vessels
    SET
        total_living_weight = total_living_weight + NEW.living_weight
    WHERE
        fiskeridir_vessel_id = _fiskeridir_vessel_id;

    INSERT INTO
        vessel_catch_aggregates (
            fiskeridir_vessel_id,
            species_group_id,
            total_living_weight
        )
    VALUES
        (
            _fiskeridir_vessel_id,
            NEW.species_group_id,
            NEW.living_weight
        )
    ON CONFLICT (fiskeridir_vessel_id, species_group_id) DO UPDATE
    SET
        total_living_weight = vessel_catch_aggregates.total_living_weight + EXCLUDED.total_living_weight;

    RETURN NULL;
  END;
$$;

CREATE OR REPLACE FUNCTION remove_from_vessel_catch_aggregates () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
  DECLARE _landing_timestamp TIMESTAMPTZ;
  DECLARE _fiskeridir_vessel_id BIGINT;
  BEGIN
    IF OLD.living_weight IS NULL THEN
        RETURN NULL;
    END IF;
    SELECT
        landing_timestamp,
        fiskeridir_vessel_id INTO _landing_timestamp,
        _fiskeridir_vessel_id
    FROM
        landings
    WHERE
        landing_id = NEW.landing_id
        AND fiskeridir_vessel_id IS NOT NULL;

    IF (_landing_timestamp IS NULL OR _landing_timestamp < '2023-01-01 00:00:00') THEN
        RETURN NULL;
    END IF;

    UPDATE fiskeridir_vessels
    SET
        total_living_weight = total_living_weight - OLD.living_weight
    WHERE
        fiskeridir_vessel_id = _fiskeridir_vessel_id;

    UPDATE vessel_catch_aggregates
    SET
        total_living_weight = total_living_weight - OLD.living_weight
    WHERE
        fiskeridir_vessel_id = _fiskeridir_vessel_id
        AND species_group_id = OLD.species_group_id;
    RETURN NULL;
  END;
$$;

CREATE TRIGGER landing_entries_after_insert_update_aggregates
AFTER INSERT ON landing_entries FOR EACH ROW
EXECUTE FUNCTION add_to_vessel_catch_aggregates ();

CREATE TRIGGER landing_entries_after_delete_update_aggregates
AFTER DELETE ON landing_entries FOR EACH ROW
EXECUTE FUNCTION remove_from_vessel_catch_aggregates ();

INSERT INTO
    vessel_catch_aggregates (
        fiskeridir_vessel_id,
        species_group_id,
        total_living_weight
    )
SELECT
    fiskeridir_vessel_id,
    species_group_id,
    SUM(living_weight)
FROM
    landing_entries le
    INNER JOIN landings l ON le.landing_id = l.landing_id
WHERE
    living_weight IS NOT NULL
    AND fiskeridir_vessel_id IS NOT NULL
    AND l.landing_timestamp >= '2023-01-01 00:00:00'
GROUP BY
    fiskeridir_vessel_id,
    species_group_id;

WITH
    sums AS (
        SELECT
            fiskeridir_vessel_id,
            SUM(living_weight) AS total_living_weight
        FROM
            landing_entries le
            INNER JOIN landings l ON le.landing_id = l.landing_id
        WHERE
            living_weight IS NOT NULL
            AND fiskeridir_vessel_id IS NOT NULL
            AND l.landing_timestamp >= '2023-01-01 00:00:00'
        GROUP BY
            fiskeridir_vessel_id
    )
UPDATE fiskeridir_vessels f
SET
    total_living_weight = s.total_living_weight
FROM
    sums s
WHERE
    s.fiskeridir_vessel_id = f.fiskeridir_vessel_id;
