DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'ers_dca_%';

DELETE FROM ers_dca;

CREATE TABLE
    hauls_matrix (
        message_id INT NOT NULL,
        start_timestamp timestamptz NOT NULL,
        stop_timestamp timestamptz NOT NULL,
        catch_location_start_matrix_index INT NOT NULL,
        catch_location_start VARCHAR NOT NULL,
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT NOT NULL,
        fiskeridir_vessel_id INT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        living_weight DECIMAL NOT NULL,
        PRIMARY KEY (
            message_id,
            start_timestamp,
            stop_timestamp,
            species_group_id
        ),
        FOREIGN KEY (message_id, start_timestamp, stop_timestamp) REFERENCES ers_dca (message_id, start_timestamp, stop_timestamp) ON DELETE CASCADE
    );

CREATE
OR REPLACE FUNCTION add_to_hauls_matrix () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.living_weight IS NOT NULL) THEN
                INSERT INTO
                    hauls_matrix (
                        message_id,
                        start_timestamp,
                        stop_timestamp,
                        catch_location_start_matrix_index,
                        catch_location_start,
                        matrix_month_bucket,
                        vessel_length_group,
                        fiskeridir_vessel_id,
                        gear_group_id,
                        species_group_id,
                        living_weight
                    )
                SELECT
                    NEW.message_id,
                    NEW.start_timestamp,
                    NEW.stop_timestamp,
                    l.matrix_index,
                    l.catch_location_id,
                    HAULS_MATRIX_MONTH_BUCKET (NEW.start_timestamp),
                    TO_VESSEL_LENGTH_GROUP (e.vessel_length) AS vessel_length_group,
                    e.fiskeridir_vessel_id,
                    e.gear_group_id,
                    NEW.species_group_id,
                    NEW.living_weight
                FROM
                    ers_dca e
                    INNER JOIN catch_locations l ON ST_CONTAINS (
                        l.polygon,
                        ST_POINT (e.start_longitude, e.start_latitude)
                    )
                WHERE
                    e.message_id = NEW.message_id
                    AND e.start_timestamp = NEW.start_timestamp
                    AND e.stop_timestamp = NEW.stop_timestamp
                    AND HAULS_MATRIX_MONTH_BUCKET (e.start_timestamp) >= 2010 * 12
                ON CONFLICT (
                    message_id,
                    start_timestamp,
                    stop_timestamp,
                    species_group_id
                ) DO
                UPDATE
                SET
                    living_weight = hauls_matrix.living_weight + excluded.living_weight;
            END IF;
        END IF;

        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION subtract_from_hauls_matrix () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            UPDATE hauls_matrix
            SET
                living_weight = hauls_matrix.living_weight - OLD.living_weight
            WHERE
                message_id = OLD.message_id
                AND start_timestamp = OLD.start_timestamp
                AND stop_timestamp = OLD.stop_timestamp
                AND species_group_id = OLD.species_group_id;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER ers_dca_catches_after_delete_subtract_from_matrix
AFTER DELETE ON ers_dca_catches FOR EACH ROW
EXECUTE FUNCTION subtract_from_hauls_matrix ();

CREATE TRIGGER ers_dca_catches_after_insert_add_to_matrix
AFTER INSERT ON ers_dca_catches FOR EACH ROW
EXECUTE FUNCTION add_to_hauls_matrix ();

CREATE INDEX ON hauls_matrix (catch_location_start_matrix_index);

CREATE INDEX ON hauls_matrix (catch_location_start);

CREATE INDEX ON hauls_matrix (matrix_month_bucket);

CREATE INDEX ON hauls_matrix (gear_group_id);

CREATE INDEX ON hauls_matrix (species_group_id);

CREATE INDEX ON hauls_matrix (fiskeridir_vessel_id);

CREATE INDEX ON hauls_matrix (vessel_length_group);

CREATE INDEX ON hauls_matrix (gear_group_id, vessel_length_group, living_weight);

CREATE INDEX ON hauls_matrix (
    gear_group_id,
    catch_location_start_matrix_index,
    living_weight
);

CREATE INDEX ON hauls_matrix (gear_group_id, matrix_month_bucket, living_weight);

CREATE INDEX ON hauls_matrix_view (gear_group_id, species_group_id, living_weight);

CREATE INDEX ON hauls_matrix (
    catch_location_start_matrix_index,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON hauls_matrix (
    catch_location_start_matrix_index,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    catch_location_start_matrix_index,
    gear_group_id,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    catch_location_start_matrix_index,
    species_group_id,
    living_weight
);

CREATE INDEX ON hauls_matrix (
    species_group_id,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON hauls_matrix (
    species_group_id,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON hauls_matrix (species_group_id, gear_group_id, living_weight);

CREATE INDEX ON hauls_matrix (
    species_group_id,
    catch_location_start_matrix_index,
    living_weight
);

CREATE INDEX ON hauls_matrix (
    vessel_length_group,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (vessel_length_group, gear_group_id, living_weight);

CREATE INDEX ON hauls_matrix_view (
    vessel_length_group,
    species_group_id,
    living_weight
);

CREATE INDEX ON hauls_matrix_view (
    vessel_length_group,
    catch_location_start_matrix_index,
    living_weight
);
