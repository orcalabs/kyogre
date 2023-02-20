CREATE TABLE
    ers_message_types (
        ers_message_type_id VARCHAR PRIMARY KEY CHECK (ers_message_type_id <> ''),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    ers_activities (
        ers_activity_id VARCHAR PRIMARY KEY CHECK (ers_activity_id <> ''),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    ers_quantum_types (
        ers_quantum_type_id VARCHAR PRIMARY KEY CHECK (ers_quantum_type_id <> ''),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    gear_problems (
        gear_problem_id INT PRIMARY KEY,
        "name" VARCHAR CHECK ("name" <> '')
    );

CREATE TABLE
    gear_specifications (
        gear_specification_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    fiskeridir_vessel_nationality_groups (
        fiskeridir_vessel_nationality_group_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    herring_populations (
        herring_population_id VARCHAR PRIMARY KEY CHECK (herring_population_id <> ''),
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    gear_fiskeridir (
        gear_fiskeridir_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    whale_genders (
        whale_gender_id INT PRIMARY KEY,
        "name" VARCHAR NOT NULL CHECK ("name" <> '')
    );

CREATE TABLE
    main_species_fao (
        "name" VARCHAR CHECK ("name" <> ''),
        main_species_fao_id VARCHAR PRIMARY KEY CHECK (main_species_fao_id <> '')
    );

CREATE TABLE
    ers_dca (
        message_id BIGINT,
        message_date date NOT NULL,
        message_number INT NOT NULL,
        message_time TIME NOT NULL,
        message_timestamp timestamptz NOT NULL,
        ers_message_type_id VARCHAR NOT NULL REFERENCES ers_message_types (ers_message_type_id),
        message_year INT NOT NULL,
        relevant_year INT NOT NULL,
        sequence_number INT,
        message_version INT NOT NULL,
        ers_activity_id VARCHAR NOT NULL REFERENCES ers_activities (ers_activity_id),
        area_grouping_end_id VARCHAR REFERENCES area_groupings (area_grouping_id),
        area_grouping_start_id VARCHAR REFERENCES area_groupings (area_grouping_id),
        call_sign_of_loading_vessel VARCHAR,
        catch_year INT,
        duration INT,
        economic_zone_id VARCHAR REFERENCES economic_zones (economic_zone_id),
        haul_distance INT,
        herring_population_id VARCHAR REFERENCES herring_populations (herring_population_id),
        herring_population_fiskeridir_id INT,
        location_end_code INT,
        location_start_code INT,
        main_area_end_id INT REFERENCES catch_main_areas (catch_main_area_id),
        main_area_start_id INT REFERENCES catch_main_areas (catch_main_area_id),
        ocean_depth_end INT,
        ocean_depth_start INT,
        quota_type_id INT NOT NULL REFERENCES quota_types (quota_type_id),
        start_date date,
        start_latitude DECIMAL,
        start_longitude DECIMAL,
        start_time TIME,
        start_timestamp timestamptz,
        stop_date date,
        stop_latitude DECIMAL,
        stop_longitude DECIMAL,
        stop_time TIME,
        stop_timestamp timestamptz,
        gear_amount INT,
        gear_fao_id VARCHAR REFERENCES gear_fao (gear_fao_id),
        gear_fiskeridir_id INT REFERENCES gear_fiskeridir (gear_fiskeridir_id),
        gear_group_id INT REFERENCES gear_groups (gear_group_id),
        gear_main_group_id INT REFERENCES gear_main_groups (gear_main_group_id),
        gear_mesh_width INT,
        gear_problem_id INT REFERENCES gear_problems (gear_problem_id),
        gear_specification_id INT REFERENCES gear_specifications (gear_specification_id),
        port_id VARCHAR REFERENCES ports (port_id),
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_building_year INT,
        vessel_call_sign VARCHAR CHECK (vessel_call_sign <> ''),
        vessel_call_sign_ers VARCHAR NOT NULL CHECK (vessel_call_sign_ers <> ''),
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_county VARCHAR CHECK (vessel_county <> ''),
        vessel_county_code INT,
        vessel_greatest_length DECIMAL,
        vessel_identification VARCHAR CHECK (vessel_identification <> ''),
        vessel_length DECIMAL NOT NULL,
        vessel_length_group VARCHAR CHECK (vessel_length_group <> ''),
        vessel_length_group_code INT,
        vessel_material_code VARCHAR CHECK (vessel_material_code <> ''),
        vessel_municipality VARCHAR CHECK (vessel_municipality <> ''),
        vessel_municipality_code INT,
        vessel_name VARCHAR CHECK (vessel_name <> ''),
        vessel_name_ers VARCHAR CHECK (vessel_name_ers <> ''),
        vessel_nationality_code VARCHAR CHECK (vessel_nationality_code <> ''),
        fiskeridir_vessel_nationality_group_id INT NOT NULL REFERENCES fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id),
        vessel_rebuilding_year INT,
        vessel_registration_id VARCHAR CHECK (vessel_registration_id <> ''),
        vessel_registration_id_ers VARCHAR CHECK (vessel_registration_id_ers <> ''),
        vessel_valid_until date,
        vessel_width DECIMAL,
        main_species_fao_id VARCHAR REFERENCES main_species_fao (main_species_fao_id),
        main_species_fiskeridir_id INT,
        living_weight INT,
        species_fao_id VARCHAR REFERENCES species_fao (species_fao_id),
        species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        species_group_id INT REFERENCES species_groups (species_group_id),
        species_main_group_id INT REFERENCES species_main_groups (species_main_group_id),
        whale_blubber_measure_a INT,
        whale_blubber_measure_b INT,
        whale_blubber_measure_c INT,
        whale_circumference INT,
        whale_fetus_length INT,
        whale_gender_id INT REFERENCES whale_genders (whale_gender_id),
        whale_grenade_number VARCHAR,
        whale_individual_number INT,
        whale_length INT
    );

CREATE TABLE
    ers_arrivals (
        message_id BIGINT PRIMARY KEY,
        message_date date NOT NULL,
        message_number INT NOT NULL,
        message_time TIME NOT NULL,
        message_timestamp timestamptz NOT NULL,
        ers_message_type_id VARCHAR NOT NULL REFERENCES ers_message_types (ers_message_type_id),
        message_year INT NOT NULL,
        relevant_year INT NOT NULL,
        sequence_number INT,
        arrival_date date NOT NULL,
        arrival_time TIME NOT NULL,
        arrival_timestamp timestamptz NOT NULL,
        landing_facility VARCHAR CHECK (landing_facility <> ''),
        port_id VARCHAR REFERENCES ports (port_id),
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_building_year INT,
        vessel_call_sign VARCHAR CHECK (vessel_call_sign <> ''),
        vessel_call_sign_ers VARCHAR NOT NULL CHECK (vessel_call_sign_ers <> ''),
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_county VARCHAR CHECK (vessel_county <> ''),
        vessel_county_code INT,
        vessel_greatest_length DECIMAL,
        vessel_identification VARCHAR CHECK (vessel_identification <> ''),
        vessel_length DECIMAL NOT NULL,
        vessel_length_group VARCHAR CHECK (vessel_length_group <> ''),
        vessel_length_group_code INT,
        vessel_material_code VARCHAR CHECK (vessel_material_code <> ''),
        vessel_municipality VARCHAR CHECK (vessel_municipality <> ''),
        vessel_municipality_code INT,
        vessel_name VARCHAR CHECK (vessel_name <> ''),
        vessel_name_ers VARCHAR CHECK (vessel_name_ers <> ''),
        vessel_nationality_code VARCHAR CHECK (vessel_nationality_code <> ''),
        fiskeridir_vessel_nationality_group_id INT NOT NULL REFERENCES fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id),
        vessel_rebuilding_year INT,
        vessel_registration_id VARCHAR CHECK (vessel_registration_id <> ''),
        vessel_registration_id_ers VARCHAR CHECK (vessel_registration_id_ers <> ''),
        vessel_valid_until date,
        vessel_width DECIMAL
    );

CREATE TABLE
    ers_departures (
        message_id BIGINT PRIMARY KEY,
        message_date date NOT NULL,
        message_number INT NOT NULL,
        message_time TIME NOT NULL,
        message_timestamp timestamptz NOT NULL,
        ers_message_type_id VARCHAR NOT NULL REFERENCES ers_message_types (ers_message_type_id),
        message_year INT NOT NULL,
        relevant_year INT NOT NULL,
        sequence_number INT,
        ers_activity_id VARCHAR REFERENCES ers_activities (ers_activity_id),
        departure_date DATE NOT NULL,
        departure_time TIME NOT NULL,
        departure_timestamp timestamptz NOT NULL,
        fishing_date date NOT NULL,
        fishing_time TIME NOT NULL,
        fishing_timestamp timestamptz NOT NULL,
        start_latitude DECIMAL NOT NULL,
        start_latitude_sggdd VARCHAR NOT NULL,
        start_longitude DECIMAL NOT NULL,
        start_longitude_sggdd VARCHAR NOT NULL,
        target_species_fao_id VARCHAR NOT NULL REFERENCES species_fao (species_fao_id),
        target_species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        port_id VARCHAR REFERENCES ports (port_id),
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_building_year INT,
        vessel_call_sign VARCHAR CHECK (vessel_call_sign <> ''),
        vessel_call_sign_ers VARCHAR NOT NULL CHECK (vessel_call_sign_ers <> ''),
        vessel_engine_building_year INT,
        vessel_engine_power INT,
        vessel_gross_tonnage_1969 INT,
        vessel_gross_tonnage_other INT,
        vessel_county VARCHAR CHECK (vessel_county <> ''),
        vessel_county_code INT,
        vessel_greatest_length DECIMAL,
        vessel_identification VARCHAR CHECK (vessel_identification <> ''),
        vessel_length DECIMAL NOT NULL,
        vessel_length_group VARCHAR CHECK (vessel_length_group <> ''),
        vessel_length_group_code INT,
        vessel_material_code VARCHAR CHECK (vessel_material_code <> ''),
        vessel_municipality VARCHAR CHECK (vessel_municipality <> ''),
        vessel_municipality_code INT,
        vessel_name VARCHAR CHECK (vessel_name <> ''),
        vessel_name_ers VARCHAR CHECK (vessel_name_ers <> ''),
        vessel_nationality_code VARCHAR CHECK (vessel_nationality_code <> ''),
        fiskeridir_vessel_nationality_group_id INT NOT NULL REFERENCES fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id),
        vessel_rebuilding_year INT,
        vessel_registration_id VARCHAR CHECK (vessel_registration_id <> ''),
        vessel_registration_id_ers VARCHAR CHECK (vessel_registration_id_ers <> ''),
        vessel_valid_until date,
        vessel_width DECIMAL
    );

CREATE TABLE
    ers_arrival_catches (
        message_id BIGINT NOT NULL REFERENCES ers_arrivals (message_id),
        ers_quantum_type_id VARCHAR NOT NULL REFERENCES ers_quantum_types (ers_quantum_type_id),
        living_weight INT NOT NULL,
        species_fao_id VARCHAR NOT NULL REFERENCES species_fao (species_fao_id),
        species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        species_group_id INT REFERENCES species_groups (species_group_id),
        species_main_group_id INT REFERENCES species_main_groups (species_main_group_id),
        PRIMARY KEY (message_id, species_fao_id)
    );

CREATE TABLE
    ers_departure_catches (
        message_id BIGINT NOT NULL REFERENCES ers_departures (message_id),
        ers_quantum_type_id VARCHAR NOT NULL REFERENCES ers_quantum_types (ers_quantum_type_id),
        living_weight INT NOT NULL,
        species_fao_id VARCHAR NOT NULL REFERENCES species_fao (species_fao_id),
        species_fiskeridir_id INT REFERENCES species_fiskeridir (species_fiskeridir_id),
        species_group_id INT REFERENCES species_groups (species_group_id),
        species_main_group_id INT REFERENCES species_main_groups (species_main_group_id),
        PRIMARY KEY (message_id, species_fao_id)
    );

INSERT INTO
    ers_quantum_types (ers_quantum_type_id, "name")
VALUES
    ('KG', 'Fangst overført'),
    ('OB', 'Fangst ombord');

INSERT INTO
    ers_activities (ers_activity_id, "name")
VALUES
    ('FIS', 'Fiske overført'),
    (
        'REL',
        'Fangst relokalisering (overføring av fangst)'
    ),
    ('SCR', 'Vitenskapelig forskning'),
    ('STE', 'Stimer'),
    ('TRX', 'Omlasting'),
    ('SET', 'Setting av redskap'),
    ('ANC', 'Ankring'),
    ('DRI', 'Driving'),
    ('GUD', 'Vaktskip'),
    ('HAU', 'Transport'),
    ('PRO', 'Produksjon'),
    ('INW', 'Ingen aktivitet'),
    ('SEF', 'Leting etter fisk'),
    ('OTH', 'Annet');

INSERT INTO
    gear_problems (gear_problem_id, "name")
VALUES
    (0, 'ukjent'),
    (1, 'bomkast'),
    (2, 'notsprenging'),
    (3, 'splitt'),
    (4, 'hull i sekk'),
    (5, 'mistet redskap'),
    (6, 'annet');

INSERT INTO
    gear_specifications (gear_specification_id, "name")
VALUES
    (1, 'enkeltrål'),
    (2, 'dobbeltrål'),
    (3, 'trippeltrål'),
    (4, 'mer enn tre tråler');

INSERT INTO
    fiskeridir_vessel_nationality_groups (fiskeridir_vessel_nationality_group_id, "name")
VALUES
    (1, 'Foreign'),
    (2, 'Norwegian'),
    (3, 'Test');

INSERT INTO
    whale_genders (whale_gender_id, "name")
VALUES
    (1, 'Hannkjønn'),
    (2, 'Hunnkjønn');

ALTER TABLE landings
ALTER COLUMN vessel_nation_group_id
DROP NOT NULL;

ALTER TABLE fiskeridir_vessels
ALTER COLUMN fiskeridir_nation_group_id
DROP NOT NULL;

ALTER TABLE species_fao
ALTER COLUMN "name"
DROP NOT NULL;

ALTER TABLE species_fiskeridir
ALTER COLUMN "name"
DROP NOT NULL;

ALTER TABLE catch_main_areas
ALTER COLUMN "name"
DROP NOT NULL;

ALTER TABLE gear_fao
ALTER COLUMN "name"
DROP NOT NULL;

CREATE MATERIALIZED VIEW
    hauls_view AS
SELECT
    e.message_id AS message_id,
    MIN(e.message_date) AS message_date,
    MIN(e.message_number) AS message_number,
    MIN(e.message_time) AS message_time,
    MIN(e.message_timestamp) AS message_timestamp,
    MIN(e.ers_message_type_id) AS ers_message_type_id,
    MIN(e.message_year) AS message_year,
    MIN(e.relevant_year) AS relevant_year,
    MIN(e.sequence_number) AS sequence_number,
    MIN(e.message_version) AS message_version,
    MIN(e.ers_activity_id) AS ers_activity_id,
    MIN(e.area_grouping_end_id) AS area_grouping_end_id,
    MIN(e.area_grouping_start_id) AS area_grouping_start_id,
    MIN(e.call_sign_of_loading_vessel) AS call_sign_of_loading_vessel,
    MIN(e.catch_year) AS catch_year,
    MIN(e.duration) AS duration,
    MIN(e.economic_zone_id) AS economic_zone_id,
    MIN(e.haul_distance) AS haul_distance,
    MIN(e.herring_population_id) AS herring_population_id,
    MIN(e.herring_population_fiskeridir_id) AS herring_population_fiskeridir_id,
    MIN(e.location_end_code) AS location_end_code,
    MIN(e.location_start_code) AS location_start_code,
    MIN(e.main_area_end_id) AS main_area_end_id,
    MIN(e.main_area_start_id) AS main_area_start_id,
    MIN(e.ocean_depth_end) AS ocean_depth_end,
    MIN(e.ocean_depth_start) AS ocean_depth_start,
    MIN(e.quota_type_id) AS quota_type_id,
    MIN(e.start_date) AS start_date,
    MIN(e.start_latitude) AS start_latitude,
    MIN(e.start_longitude) AS start_longitude,
    MIN(e.start_time) AS start_time,
    MIN(e.start_timestamp) AS start_timestamp,
    MIN(e.stop_date) AS stop_date,
    MIN(e.stop_latitude) AS stop_latitude,
    MIN(e.stop_longitude) AS stop_longitude,
    MIN(e.stop_time) AS stop_time,
    MIN(e.stop_timestamp) AS stop_timestamp,
    MIN(e.gear_amount) AS gear_amount,
    MIN(e.gear_fao_id) AS gear_fao_id,
    MIN(e.gear_fiskeridir_id) AS gear_fiskeridir_id,
    MIN(e.gear_group_id) AS gear_group_id,
    MIN(e.gear_main_group_id) AS gear_main_group_id,
    MIN(e.gear_mesh_width) AS gear_mesh_width,
    MIN(e.gear_problem_id) AS gear_problem_id,
    MIN(e.gear_specification_id) AS gear_specification_id,
    MIN(e.port_id) AS port_id,
    MIN(e.fiskeridir_vessel_id) AS fiskeridir_vessel_id,
    MIN(e.vessel_building_year) AS vessel_building_year,
    MIN(e.vessel_call_sign) AS vessel_call_sign,
    MIN(e.vessel_call_sign_ers) AS vessel_call_sign_ers,
    MIN(e.vessel_engine_building_year) AS vessel_engine_building_year,
    MIN(e.vessel_engine_power) AS vessel_engine_power,
    MIN(e.vessel_gross_tonnage_1969) AS vessel_gross_tonnage_1969,
    MIN(e.vessel_gross_tonnage_other) AS vessel_gross_tonnage_other,
    MIN(e.vessel_county) AS vessel_county,
    MIN(e.vessel_county_code) AS vessel_county_code,
    MIN(e.vessel_greatest_length) AS vessel_greatest_length,
    MIN(e.vessel_identification) AS vessel_identification,
    MIN(e.vessel_length) AS vessel_length,
    MIN(e.vessel_length_group) AS vessel_length_group,
    MIN(e.vessel_length_group_code) AS vessel_length_group_code,
    MIN(e.vessel_material_code) AS vessel_material_code,
    MIN(e.vessel_municipality) AS vessel_municipality,
    MIN(e.vessel_municipality_code) AS vessel_municipality_code,
    MIN(e.vessel_name) AS vessel_name,
    MIN(e.vessel_name_ers) AS vessel_name_ers,
    MIN(e.vessel_nationality_code) AS vessel_nationality_code,
    MIN(e.fiskeridir_vessel_nationality_group_id) AS vessel_nationality_group_id,
    MIN(e.vessel_rebuilding_year) AS vessel_rebuilding_year,
    MIN(e.vessel_registration_id) AS vessel_registration_id,
    MIN(e.vessel_registration_id_ers) AS vessel_registration_id_ers,
    MIN(e.vessel_valid_until) AS vessel_valid_until,
    MIN(e.vessel_width) AS vessel_width,
    COALESCE(
        JSON_AGG(
            JSON_BUILD_OBJECT(
                'main_species_fao_id',
                e.main_species_fao_id,
                'main_species_fiskerid',
                e.main_species_fiskeridir_id,
                'living_weight',
                e.living_weight,
                'species_fao_id',
                e.species_fao_id,
                'species_fiskeridir_id',
                e.species_fiskeridir_id,
                'species_group_id',
                e.species_group_id,
                'species_main_group_id',
                e.species_main_group_id
            )
        ) FILTER (
            WHERE
                e.main_species_fao_id IS NOT NULL
                OR e.main_species_fiskeridir_id IS NOT NULL
                OR e.living_weight IS NOT NULL
                OR e.species_fao_id IS NOT NULL
                OR e.species_fiskeridir_id IS NOT NULL
                OR e.species_group_id IS NOT NULL
                OR e.species_main_group_id IS NOT NULL
        ),
        '[]'
    ) AS catches,
    COALESCE(
        JSON_AGG(
            JSON_BUILD_OBJECT(
                'blubber_measure_a',
                e.whale_blubber_measure_a,
                'blubber_measure_b',
                e.whale_blubber_measure_b,
                'blubber_measure_c',
                e.whale_blubber_measure_c,
                'circumference',
                e.whale_circumference,
                'fetus_length',
                e.whale_fetus_length,
                'gender_id',
                e.whale_gender_id,
                'grenade_number',
                e.whale_grenade_number,
                'individual_number',
                e.whale_individual_number,
                'length',
                e.whale_length
            )
        ) FILTER (
            WHERE
                e.whale_blubber_measure_a IS NOT NULL
                OR e.whale_blubber_measure_b IS NOT NULL
                OR e.whale_blubber_measure_c IS NOT NULL
                OR e.whale_circumference IS NOT NULL
                OR e.whale_fetus_length IS NOT NULL
                OR e.whale_gender_id IS NOT NULL
                OR e.whale_grenade_number IS NOT NULL
                OR e.whale_individual_number IS NOT NULL
                OR e.whale_length IS NOT NULL
        ),
        '[]'
    ) AS whale_catches
FROM
    ers_dca e
WHERE
    e.main_species_fao_id IS NOT NULL
    OR e.main_species_fiskeridir_id IS NOT NULL
    OR e.living_weight IS NOT NULL
    OR e.species_fao_id IS NOT NULL
    OR e.species_fiskeridir_id IS NOT NULL
    OR e.species_group_id IS NOT NULL
    OR e.species_main_group_id IS NOT NULL
    OR e.whale_blubber_measure_b IS NOT NULL
    OR e.whale_blubber_measure_c IS NOT NULL
    OR e.whale_circumference IS NOT NULL
    OR e.whale_fetus_length IS NOT NULL
    OR e.whale_gender_id IS NOT NULL
    OR e.whale_grenade_number IS NOT NULL
    OR e.whale_individual_number IS NOT NULL
    OR e.whale_length IS NOT NULL
GROUP BY
    e.message_id,
    e.start_timestamp,
    e.stop_timestamp;

CREATE UNIQUE INDEX ON hauls_view (message_id, start_timestamp, stop_timestamp);

CREATE
OR REPLACE FUNCTION update_database_views () RETURNS void LANGUAGE PLPGSQL AS $$
    DECLARE v varchar;
    BEGIN
        FOR v IN SELECT matviewname FROM pg_matviews
        LOOP
            EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY ' || v;
        END LOOP;
    END
$$;

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('UpdateDatabaseViews');

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Scrape', 'UpdateDatabaseViews'),
    ('Pending', 'UpdateDatabaseViews'),
    ('UpdateDatabaseViews', 'Pending');
