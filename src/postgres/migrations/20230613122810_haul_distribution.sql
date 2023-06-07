DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'ers_dca_%';

TRUNCATE TABLE ers_dca CASCADE;

CREATE TABLE
    haul_distributors (
        haul_distributor_id INT PRIMARY KEY,
        description TEXT NOT NULL
    );

INSERT INTO
    haul_distributors (haul_distributor_id, description)
VALUES
    (1, 'ais_vms');

ALTER TABLE hauls
ADD COLUMN catch_locations TEXT[];

CREATE INDEX ON hauls USING GIN (catch_locations);

ALTER TABLE hauls_matrix
DROP CONSTRAINT hauls_matrix_pkey;

ALTER TABLE hauls_matrix
RENAME COLUMN catch_location_start TO catch_location;

ALTER TABLE hauls_matrix
RENAME COLUMN catch_location_start_matrix_index TO catch_location_matrix_index;

ALTER TABLE hauls_matrix
ADD COLUMN haul_distributor_id INT REFERENCES haul_distributors (haul_distributor_id);

ALTER TABLE hauls_matrix
ALTER COLUMN living_weight
TYPE BIGINT;

ALTER TABLE hauls_matrix
ADD PRIMARY KEY (
    message_id,
    start_timestamp,
    stop_timestamp,
    species_group_id,
    catch_location
);

CREATE
OR REPLACE FUNCTION add_to_hauls_matrix () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.living_weight IS NOT NULL) THEN
                DELETE FROM hauls_matrix
                WHERE
                    message_id = NEW.message_id
                    AND start_timestamp = NEW.start_timestamp
                    AND stop_timestamp = NEW.stop_timestamp;

                INSERT INTO
                    hauls_matrix (
                        message_id,
                        start_timestamp,
                        stop_timestamp,
                        catch_location_matrix_index,
                        catch_location,
                        matrix_month_bucket,
                        vessel_length_group,
                        fiskeridir_vessel_id,
                        gear_group_id,
                        species_group_id,
                        living_weight
                    )
                SELECT
                    e.message_id,
                    e.start_timestamp,
                    e.stop_timestamp,
                    l.matrix_index,
                    l.catch_location_id,
                    HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp),
                    TO_VESSEL_LENGTH_GROUP (e.vessel_length) AS vessel_length_group,
                    e.fiskeridir_vessel_id,
                    e.gear_group_id,
                    c.species_group_id,
                    SUM(c.living_weight)
                FROM
                    ers_dca e
                    INNER JOIN ers_dca_catches c ON
                        e.message_id = c.message_id
                        AND e.start_timestamp = c.start_timestamp
                        AND e.stop_timestamp = c.stop_timestamp
                    INNER JOIN catch_locations l ON ST_CONTAINS (
                        l.polygon,
                        ST_POINT (e.start_longitude, e.start_latitude)
                    )
                WHERE
                    e.message_id = NEW.message_id
                    AND e.start_timestamp = NEW.start_timestamp
                    AND e.stop_timestamp = NEW.stop_timestamp
                    AND HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp) >= 2010 * 12
            GROUP BY
                e.message_id,
                e.start_timestamp,
                e.stop_timestamp,
                c.species_group_id,
                l.catch_location_id;
            END IF;
        END IF;

        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION subtract_from_hauls_matrix () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            DELETE FROM hauls_matrix
            WHERE
                message_id = NEW.message_id
                AND start_timestamp = NEW.start_timestamp
                AND stop_timestamp = NEW.stop_timestamp;

            INSERT INTO
                hauls_matrix (
                    message_id,
                    start_timestamp,
                    stop_timestamp,
                    catch_location_matrix_index,
                    catch_location,
                    matrix_month_bucket,
                    vessel_length_group,
                    fiskeridir_vessel_id,
                    gear_group_id,
                    species_group_id,
                    living_weight
                )
            SELECT
                e.message_id,
                e.start_timestamp,
                e.stop_timestamp,
                l.matrix_index,
                l.catch_location_id,
                HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp),
                TO_VESSEL_LENGTH_GROUP (e.vessel_length) AS vessel_length_group,
                e.fiskeridir_vessel_id,
                e.gear_group_id,
                c.species_group_id,
                SUM(c.living_weight)
            FROM
                ers_dca e
                INNER JOIN ers_dca_catches c ON
                    e.message_id = c.message_id
                    AND e.start_timestamp = c.start_timestamp
                    AND e.stop_timestamp = c.stop_timestamp
                INNER JOIN catch_locations l ON ST_CONTAINS (
                    l.polygon,
                    ST_POINT (e.start_longitude, e.start_latitude)
                )
            WHERE
                e.message_id = OLD.message_id
                AND e.start_timestamp = OLD.start_timestamp
                AND e.stop_timestamp = OLD.stop_timestamp
                AND HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp) >= 2010 * 12
            GROUP BY
                e.message_id,
                e.start_timestamp,
                e.stop_timestamp,
                c.species_group_id,
                l.catch_location_id;
        END IF;
        RETURN NEW;
   END;
$$;

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('HaulDistribution');

DELETE FROM valid_engine_transitions
WHERE
    source = 'Benchmark'
    AND destination = 'Pending';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Pending', 'Benchmark'),
    ('Pending', 'HaulDistribution'),
    ('Benchmark', 'HaulDistribution'),
    ('HaulDistribution', 'Pending');

CREATE
OR REPLACE FUNCTION remove_catch_from_haul () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE
        _count INT;
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            SELECT
                JSONB_ARRAY_LENGTH(catches) + JSONB_ARRAY_LENGTH(whale_catches)
            INTO
                _count
            FROM
                hauls
            WHERE
                message_id = OLD.message_id
                AND start_timestamp = OLD.start_timestamp
                AND stop_timestamp = OLD.stop_timestamp;

            IF (_count = 1) THEN
                DELETE FROM hauls
                WHERE
                    message_id = OLD.message_id
                    AND start_timestamp = OLD.start_timestamp
                    AND stop_timestamp = OLD.stop_timestamp;
            ELSE
                UPDATE hauls
                SET
                    catches = (
                        SELECT
                            JSONB_AGG(c)
                        FROM
                            JSONB_ARRAY_ELEMENTS(hauls.catches) c
                        WHERE
                            c['species_fao_id']::TEXT != OLD.species_fao_id
                    )
                WHERE
                    message_id = OLD.message_id
                    AND start_timestamp = OLD.start_timestamp
                    AND stop_timestamp = OLD.stop_timestamp;
            END IF;
        END IF;
    RETURN NEW;
    END;
$$;

CREATE
OR REPLACE FUNCTION remove_whale_catch_from_haul () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE
        _count INT;
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            SELECT
                JSONB_ARRAY_LENGTH(catches) + JSONB_ARRAY_LENGTH(whale_catches)
            INTO
                _count
            FROM
                hauls
            WHERE
                message_id = OLD.message_id
                AND start_timestamp = OLD.start_timestamp
                AND stop_timestamp = OLD.stop_timestamp;

            IF (_count = 1) THEN
                DELETE FROM hauls
                WHERE
                    message_id = OLD.message_id
                    AND start_timestamp = OLD.start_timestamp
                    AND stop_timestamp = OLD.stop_timestamp;
            ELSE
                UPDATE hauls
                SET
                    whale_catches = (
                        SELECT
                            JSONB_AGG(w)
                        FROM
                            JSONB_ARRAY_ELEMENTS(hauls.whale_catches) w
                        WHERE
                            w['grenade_number']::TEXT != OLD.whale_grenade_number
                    )
                WHERE
                    message_id = OLD.message_id
                    AND start_timestamp = OLD.start_timestamp
                    AND stop_timestamp = OLD.stop_timestamp;
            END IF;
        END IF;
    RETURN NEW;
    END;
$$;
