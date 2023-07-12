CREATE TABLE
    landing_matrix (
        landing_id VARCHAR NOT NULL,
        catch_location_matrix_index INT NOT NULL,
        catch_location_id VARCHAR NOT NULL REFERENCES catch_locations (catch_location_id),
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT REFERENCES fiskeridir_length_groups (fiskeridir_length_group_id),
        fiskeridir_vessel_id INT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        living_weight DECIMAL NOT NULL,
        FOREIGN KEY (landing_id) REFERENCES landings (landing_id) ON DELETE CASCADE,
        PRIMARY KEY (landing_id, species_group_id)
    );

REVOKE
UPDATE ON public.landing_entries
FROM
    public;

CREATE
OR REPLACE FUNCTION add_to_landing_matrix () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.living_weight IS NOT NULL) THEN
                INSERT INTO
                    landing_matrix (
                        landing_id,
                        catch_location_id,
                        catch_location_matrix_index,
                        matrix_month_bucket,
                        vessel_length_group,
                        fiskeridir_vessel_id,
                        gear_group_id,
                        species_group_id,
                        living_weight
                    )
                SELECT
                    NEW.landing_id,
                    cl.catch_location_id,
                    cl.matrix_index,
                    HAULS_MATRIX_MONTH_BUCKET (l.landing_timestamp),
                    l.vessel_length_group_id,
                    l.fiskeridir_vessel_id,
                    l.gear_group_id,
                    NEW.species_group_id,
                    NEW.living_weight
                FROM
                    landing_entries e
                    INNER JOIN landings l ON l.landing_id = NEW.landing_id
                    INNER JOIN catch_locations cl ON l.catch_main_area_id::TEXT || '-' || l.catch_area_id::TEXT = cl.catch_location_id
                WHERE
                    e.landing_id = NEW.landing_id
                    AND e.line_number = NEW.line_number
                    AND HAULS_MATRIX_MONTH_BUCKET (l.landing_timestamp) >= 1999 * 12
                ON CONFLICT (landing_id, species_group_id) DO
                UPDATE
                SET
                    living_weight = landing_matrix.living_weight + excluded.living_weight;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION subtract_from_landing_matrix () RETURNS TRIGGER LANGUAGE PLPGSQL AS $$
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            UPDATE landing_matrix
            SET
                living_weight = landing_matrix.living_weight - OLD.living_weight
            WHERE
                landing_id = OLD.landing_id
                AND species_group_id = OLD.species_group_id;
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER landing_entries_after_delete_subtract_from_matrix
AFTER DELETE ON landing_entries FOR EACH ROW
EXECUTE FUNCTION subtract_from_landing_matrix ();

CREATE TRIGGER landing_entries_after_insert_add_to_matrix
AFTER INSERT ON landing_entries FOR EACH ROW
EXECUTE FUNCTION add_to_landing_matrix ();

CREATE INDEX ON landing_matrix (catch_location_matrix_index);

CREATE INDEX ON landing_matrix (catch_location_id);

CREATE INDEX ON landing_matrix (matrix_month_bucket);

CREATE INDEX ON landing_matrix (vessel_length_group);

CREATE INDEX ON landing_matrix (fiskeridir_vessel_id);

CREATE INDEX ON landing_matrix (gear_group_id);

CREATE INDEX ON landing_matrix (species_group_id);

--catch_location_matrix_index
CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    gear_group_id,
    living_weight
);

CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    species_group_id,
    living_weight
);

--matrix_month_bucket
CREATE INDEX ON landing_matrix (
    matrix_month_bucket,
    catch_location_matrix_index,
    living_weight
);

CREATE INDEX ON landing_matrix (
    matrix_month_bucket,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON landing_matrix (matrix_month_bucket, gear_group_id, living_weight);

CREATE INDEX ON landing_matrix (
    matrix_month_bucket,
    species_group_id,
    living_weight
);

--catch_location_matrix_index
CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    vessel_length_group,
    living_weight
);

CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    gear_group_id,
    living_weight
);

CREATE INDEX ON landing_matrix (
    catch_location_matrix_index,
    species_group_id,
    living_weight
);

--vessel_length_group
CREATE INDEX ON landing_matrix (
    vessel_length_group,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON landing_matrix (
    vessel_length_group,
    catch_location_matrix_index,
    living_weight
);

CREATE INDEX ON landing_matrix (vessel_length_group, gear_group_id, living_weight);

CREATE INDEX ON landing_matrix (
    vessel_length_group,
    species_group_id,
    living_weight
);

--species_group_id
CREATE INDEX ON landing_matrix (
    species_group_id,
    matrix_month_bucket,
    living_weight
);

CREATE INDEX ON landing_matrix (
    species_group_id,
    catch_location_matrix_index,
    living_weight
);

CREATE INDEX ON landing_matrix (species_group_id, gear_group_id, living_weight);

CREATE INDEX ON landing_matrix (
    species_group_id,
    vessel_length_group,
    living_weight
);
