ALTER TABLE ers_departure_catches
ADD PRIMARY KEY (message_id, ers_quantum_type_id, species_fao_id),
ADD CONSTRAINT check_ers_quantum_type_id CHECK (ers_quantum_type_id = 'OB');

ALTER TABLE ers_arrival_catches
ADD PRIMARY KEY (message_id, ers_quantum_type_id, species_fao_id);

ALTER TABLE ers_tra_catches
ADD PRIMARY KEY (message_id, ers_quantum_type_id, species_fao_id);

TRUNCATE TABLE hauls,
hauls_matrix CASCADE;

ALTER TABLE hauls
DROP COLUMN vessel_event_type_id,
DROP CONSTRAINT hauls_message_id_start_timestamp_stop_timestamp_fkey,
DROP CONSTRAINT hauls_message_id_start_timestamp_stop_timestamp_key,
DROP CONSTRAINT hauls_check;

ALTER TABLE hauls_matrix
DROP CONSTRAINT hauls_matrix_message_id_start_timestamp_stop_timestamp_fkey,
DROP COLUMN message_id,
DROP COLUMN start_timestamp,
DROP COLUMN stop_timestamp;

DROP TABLE ers_dca,
ers_dca_other,
ers_dca_catches,
ers_dca_whale_catches;

DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'ers_dca_%';

INSERT INTO
    vessel_event_types (vessel_event_type_id, description)
VALUES
    (6, 'haul');

CREATE TABLE
    ers_dca (
        message_id BIGINT PRIMARY KEY,
        message_number INT NOT NULL,
        message_timestamp timestamptz NOT NULL,
        ers_message_type_id TEXT NOT NULL REFERENCES ers_message_types (ers_message_type_id),
        message_version INT NOT NULL,
        message_year INT NOT NULL,
        relevant_year INT NOT NULL,
        sequence_number INT,
        ers_activity_id TEXT NOT NULL REFERENCES ers_activities (ers_activity_id),
        quota_type_id INT NOT NULL REFERENCES quota_types (quota_type_id),
        port_id TEXT REFERENCES ports (port_id),
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_building_year INT,
        vessel_call_sign TEXT CHECK (vessel_call_sign <> ''),
        vessel_call_sign_ers TEXT NOT NULL CHECK (vessel_call_sign_ers <> ''),
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_county TEXT CHECK (vessel_county <> ''),
        vessel_county_code INT,
        vessel_greatest_length DECIMAL,
        vessel_identification TEXT CHECK (vessel_identification <> ''),
        vessel_length DECIMAL NOT NULL,
        vessel_length_group TEXT CHECK (vessel_length_group <> ''),
        vessel_length_group_code INT,
        vessel_material_code TEXT CHECK (vessel_material_code <> ''),
        vessel_municipality TEXT CHECK (vessel_municipality <> ''),
        vessel_municipality_code INT,
        vessel_name TEXT CHECK (vessel_name <> ''),
        vessel_name_ers TEXT CHECK (vessel_name_ers <> ''),
        vessel_nationality_code TEXT NOT NULL CHECK (vessel_nationality_code <> ''),
        fiskeridir_vessel_nationality_group_id INT NOT NULL REFERENCES fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id),
        vessel_rebuilding_year INT,
        vessel_registration_id TEXT CHECK (vessel_registration_id <> ''),
        vessel_registration_id_ers TEXT CHECK (vessel_registration_id_ers <> ''),
        vessel_valid_from DATE,
        vessel_valid_until DATE,
        vessel_width DECIMAL,
        vessel_event_type_id INT NOT NULL DEFAULT (2) CHECK (vessel_event_type_id = 2) REFERENCES vessel_event_types (vessel_event_type_id),
        vessel_event_id BIGINT UNIQUE CHECK (
            (
                vessel_event_id IS NULL
                AND fiskeridir_vessel_id IS NULL
            )
            OR (
                vessel_event_id IS NOT NULL
                AND fiskeridir_vessel_id IS NOT NULL
            )
        ),
        UNIQUE (message_id, message_version),
        FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id)
    );

CREATE INDEX ON ers_dca (relevant_year);

CREATE TABLE
    ers_dca_bodies (
        message_id BIGINT NOT NULL REFERENCES ers_dca (message_id) ON DELETE CASCADE,
        start_latitude DECIMAL,
        start_longitude DECIMAL,
        start_timestamp TIMESTAMPTZ,
        stop_latitude DECIMAL,
        stop_longitude DECIMAL,
        stop_timestamp TIMESTAMPTZ,
        ocean_depth_end INT,
        ocean_depth_start INT,
        location_end_code INT REFERENCES catch_areas (catch_area_id),
        location_start_code INT REFERENCES catch_areas (catch_area_id),
        area_grouping_end_id TEXT REFERENCES area_groupings (area_grouping_id),
        area_grouping_start_id TEXT REFERENCES area_groupings (area_grouping_id),
        main_area_end_id INT REFERENCES catch_main_areas (catch_main_area_id),
        main_area_start_id INT REFERENCES catch_main_areas (catch_main_area_id),
        duration INT,
        haul_distance INT,
        call_sign_of_loading_vessel TEXT,
        catch_year INT,
        economic_zone_id TEXT REFERENCES economic_zones (economic_zone_id),
        gear_amount INT,
        gear_id INT NOT NULL REFERENCES gear (gear_id),
        gear_fao_id TEXT REFERENCES gear_fao (gear_fao_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        gear_main_group_id INT NOT NULL REFERENCES gear_main_groups (gear_main_group_id),
        gear_mesh_width INT,
        gear_problem_id INT REFERENCES gear_problems (gear_problem_id),
        gear_specification_id INT REFERENCES gear_specifications (gear_specification_id),
        herring_population_id TEXT REFERENCES herring_populations (herring_population_id),
        herring_population_fiskeridir_id INT,
        majority_species_fao_id TEXT REFERENCES species_fao (species_fao_id),
        majority_species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        living_weight INT,
        species_fao_id TEXT REFERENCES species_fao (species_fao_id),
        species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        species_main_group_id INT NOT NULL REFERENCES species_main_groups (species_main_group_id),
        whale_grenade_number TEXT,
        whale_blubber_measure_a INT,
        whale_blubber_measure_b INT,
        whale_blubber_measure_c INT,
        whale_circumference INT,
        whale_fetus_length INT,
        whale_gender_id INT REFERENCES whale_genders (whale_gender_id),
        whale_individual_number INT,
        whale_length INT
    );

CREATE INDEX ON ers_dca_bodies (message_id);

CREATE INDEX ON ers_dca_bodies (species_fao_id, whale_grenade_number, gear_id);

ALTER TABLE hauls
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (6) CHECK (vessel_event_type_id = 6) REFERENCES vessel_event_types (vessel_event_type_id),
ADD COLUMN message_timestamp TIMESTAMPTZ NOT NULL,
ADD CONSTRAINT hauls_message_id_fkey FOREIGN KEY (message_id) REFERENCES ers_dca (message_id) ON DELETE CASCADE,
ADD CONSTRAINT hauls_unique UNIQUE (
    message_id,
    start_timestamp,
    stop_timestamp,
    start_latitude,
    start_longitude,
    stop_latitude,
    stop_longitude,
    duration,
    haul_distance,
    gear_id
);

CREATE INDEX ON hauls (message_id);

ALTER TABLE hauls_matrix
ALTER COLUMN living_weight
TYPE DOUBLE PRECISION,
ADD COLUMN haul_id BIGINT REFERENCES hauls (haul_id) ON DELETE CASCADE,
ADD PRIMARY KEY (haul_id, species_group_id, catch_location);

DROP TRIGGER vessel_events_before_inserst_connect_to_trip ON vessel_events;

DROP FUNCTION connect_trip_to_events;

DROP TRIGGER vessel_events_add_trip_assembler_conflict ON vessel_events;

DROP FUNCTION add_trip_assembler_conflict;

DROP FUNCTION ers_dca_catches_delete_old_version_number;

DROP FUNCTION add_catch_to_haul;

DROP FUNCTION remove_catch_from_haul;

DROP FUNCTION add_to_hauls_matrix;

DROP FUNCTION subtract_from_hauls_matrix;

DROP FUNCTION ers_dca_whale_catches_delete_old_version_number;

DROP FUNCTION add_whale_catch_to_haul;

DROP FUNCTION remove_whale_catch_from_haul;

DROP TRIGGER landing_entries_after_insert_add_to_matrix ON landing_entries;

DROP FUNCTION add_to_landing_matrix;

DROP TRIGGER landing_entries_after_delete_subtract_from_matrix ON landing_entries;

DROP FUNCTION subtract_from_landing_matrix;

DROP TRIGGER trips_after_insert_connect_to_events ON trips;

DROP FUNCTION connect_events_to_trip;

CREATE
OR REPLACE FUNCTION check_landing_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE
        _current_version_number int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            SELECT
                "version"
            FROM
                landings
            INTO
                _current_version_number
            WHERE
                landing_id = NEW.landing_id;

            IF _current_version_number = NEW.version THEN
                RETURN NULL;
            END IF;
        END IF;

        RETURN NEW;
    END;
$$;

CREATE
OR REPLACE FUNCTION check_ers_dca_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    DECLARE
        _current_version_number int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            SELECT
                message_version
            FROM
                ers_dca
            INTO
                _current_version_number
            WHERE
                message_id = NEW.message_id;

            IF _current_version_number = NEW.message_version THEN
                RETURN NULL;
            END IF;
        END IF;

        RETURN NEW;
    END;
$$;

CREATE
OR REPLACE FUNCTION add_ers_dca_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(2, NEW.fiskeridir_vessel_id, NULL, NEW.message_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE
OR REPLACE FUNCTION add_haul_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(6, NEW.fiskeridir_vessel_id, NEW.start_timestamp, NEW.message_timestamp);
        END IF;
        RETURN NEW;
   END;
$$;

CREATE TRIGGER ers_dca_before_insert_add_vessel_event BEFORE INSERT ON ers_dca FOR EACH ROW
EXECUTE FUNCTION add_ers_dca_vessel_event ();

CREATE TRIGGER hauls_before_insert_add_vessel_event BEFORE INSERT ON hauls FOR EACH ROW
EXECUTE FUNCTION add_haul_vessel_event ();

CREATE TRIGGER ers_dca_after_delete_remove_event
AFTER DELETE ON ers_dca FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER haul_after_delete_remove_event
AFTER DELETE ON ers_dca FOR EACH ROW
EXECUTE FUNCTION delete_vessel_event ();

CREATE TRIGGER a_ers_dca_before_insert_check_version BEFORE INSERT ON ers_dca FOR EACH ROW
EXECUTE PROCEDURE check_ers_dca_version ();

CREATE TRIGGER ers_dca_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON ers_dca FOR EACH STATEMENT
EXECUTE FUNCTION delete_vessel_events (2);

CREATE TRIGGER hauls_after_truncate_delete_vessel_events
AFTER
TRUNCATE ON ers_dca FOR EACH STATEMENT
EXECUTE FUNCTION delete_vessel_events (6);
