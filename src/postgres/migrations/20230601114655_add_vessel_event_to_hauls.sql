ALTER TABLE hauls
ADD COLUMN vessel_event_type_id INT NOT NULL DEFAULT (2) CHECK (vessel_event_type_id = 2) REFERENCES vessel_event_types (vessel_event_type_id);

ALTER TABLE hauls
ADD COLUMN vessel_event_id BIGINT UNIQUE;

UPDATE hauls
SET
    vessel_event_id = e.vessel_event_id
FROM
    ers_dca e
WHERE
    hauls.message_id = e.message_id
    AND hauls.start_timestamp = e.start_timestamp
    AND hauls.stop_timestamp = e.stop_timestamp;

ALTER TABLE hauls
ADD CONSTRAINT hauls_vessel_event_id_and_fiskeridir_vessel_id_check CHECK (
    (
        vessel_event_id IS NULL
        AND fiskeridir_vessel_id IS NULL
    )
    OR (
        vessel_event_id IS NOT NULL
        AND fiskeridir_vessel_id IS NOT NULL
    )
);

ALTER TABLE hauls
ADD CONSTRAINT vessel_event_fk FOREIGN KEY (vessel_event_id, vessel_event_type_id) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id);

CREATE INDEX ON trips (fiskeridir_vessel_id, "period");

CREATE INDEX ON landings (gear_group_id);

CREATE INDEX ON landings (delivery_point_id);

CREATE
OR REPLACE FUNCTION add_catch_to_haul () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                hauls (
                    message_id,
                    start_timestamp,
                    stop_timestamp,
                    ers_activity_id,
                    vessel_event_id,
                    vessel_event_type_id,
                    duration,
                    haul_distance,
                    ocean_depth_end,
                    ocean_depth_start,
                    quota_type_id,
                    start_latitude,
                    start_longitude,
                    stop_latitude,
                    stop_longitude,
                    fiskeridir_vessel_id,
                    vessel_call_sign,
                    vessel_call_sign_ers,
                    vessel_name,
                    vessel_name_ers,
                    vessel_length,
                    catch_location_start,
                    gear_id,
                    gear_group_id,
                    catches
                )
            SELECT
                e.message_id,
                e.start_timestamp,
                e.stop_timestamp,
                e.ers_activity_id,
                e.vessel_event_id,
                e.vessel_event_type_id,
                e.duration,
                e.haul_distance,
                e.ocean_depth_end,
                e.ocean_depth_start,
                e.quota_type_id,
                e.start_latitude,
                e.start_longitude,
                e.stop_latitude,
                e.stop_longitude,
                e.fiskeridir_vessel_id,
                e.vessel_call_sign,
                e.vessel_call_sign_ers,
                e.vessel_name,
                e.vessel_name_ers,
                e.vessel_length,
                l.catch_location_id,
                e.gear_id,
                e.gear_group_id,
                JSONB_BUILD_ARRAY(
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(NEW.living_weight, 0),
                        'species_fao_id',
                        NEW.species_fao_id,
                        'species_fiskeridir_id',
                        NEW.species_fiskeridir_id,
                        'species_group_id',
                        NEW.species_group_id,
                        'species_main_group_id',
                        NEW.species_main_group_id
                    )
                )
            FROM
                ers_dca e
                LEFT JOIN catch_locations l ON ST_CONTAINS (
                    l.polygon,
                    ST_POINT (e.start_longitude, e.start_latitude)
                )
            WHERE
                e.message_id = NEW.message_id
                AND e.start_timestamp = NEW.start_timestamp
                AND e.stop_timestamp = NEW.stop_timestamp
            ON CONFLICT (message_id, start_timestamp, stop_timestamp) DO
            UPDATE
            SET
                catches = hauls.catches || JSONB_BUILD_OBJECT(
                    'living_weight',
                    COALESCE(NEW.living_weight, 0),
                    'species_fao_id',
                    NEW.species_fao_id,
                    'species_fiskeridir_id',
                    NEW.species_fiskeridir_id,
                    'species_group_id',
                    NEW.species_group_id,
                    'species_main_group_id',
                    NEW.species_main_group_id
                );
        END IF;

        RETURN NEW;
    END;
$$;

CREATE
OR REPLACE FUNCTION add_whale_catch_to_haul () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                hauls (
                    message_id,
                    start_timestamp,
                    stop_timestamp,
                    ers_activity_id,
                    vessel_event_id,
                    vessel_event_type_id,
                    duration,
                    haul_distance,
                    ocean_depth_end,
                    ocean_depth_start,
                    quota_type_id,
                    start_latitude,
                    start_longitude,
                    stop_latitude,
                    stop_longitude,
                    fiskeridir_vessel_id,
                    vessel_call_sign,
                    vessel_call_sign_ers,
                    vessel_name,
                    vessel_name_ers,
                    vessel_length,
                    catch_location_start,
                    gear_id,
                    gear_group_id,
                    whale_catches
                )
            SELECT
                e.message_id,
                e.start_timestamp,
                e.stop_timestamp,
                e.ers_activity_id,
                e.vessel_event_id,
                e.vessel_event_type_id,
                e.duration,
                e.haul_distance,
                e.ocean_depth_end,
                e.ocean_depth_start,
                e.quota_type_id,
                e.start_latitude,
                e.start_longitude,
                e.stop_latitude,
                e.stop_longitude,
                e.fiskeridir_vessel_id,
                e.vessel_call_sign,
                e.vessel_call_sign_ers,
                e.vessel_name,
                e.vessel_name_ers,
                e.vessel_length,
                l.catch_location_id,
                e.gear_id,
                e.gear_group_id,
                JSONB_BUILD_ARRAY(
                    JSONB_BUILD_OBJECT(
                        'grenade_number',
                        NEW.whale_grenade_number,
                        'blubber_measure_a',
                        NEW.whale_blubber_measure_a,
                        'blubber_measure_b',
                        NEW.whale_blubber_measure_b,
                        'blubber_measure_c',
                        NEW.whale_blubber_measure_c,
                        'circumference',
                        NEW.whale_circumference,
                        'fetus_length',
                        NEW.whale_fetus_length,
                        'gender_id',
                        NEW.whale_gender_id,
                        'individual_number',
                        NEW.whale_individual_number,
                        'length',
                        NEW.whale_length
                    )
                )
            FROM
                ers_dca e
                LEFT JOIN catch_locations l ON ST_CONTAINS (
                    l.polygon,
                    ST_POINT (e.start_longitude, e.start_latitude)
                )
            WHERE
                e.message_id = NEW.message_id
                AND e.start_timestamp = NEW.start_timestamp
                AND e.stop_timestamp = NEW.stop_timestamp
            ON CONFLICT (message_id, start_timestamp, stop_timestamp) DO
            UPDATE
            SET
                whale_catches = hauls.whale_catches || JSONB_BUILD_OBJECT(
                    'grenade_number',
                    NEW.whale_grenade_number,
                    'blubber_measure_a',
                    NEW.whale_blubber_measure_a,
                    'blubber_measure_b',
                    NEW.whale_blubber_measure_b,
                    'blubber_measure_c',
                    NEW.whale_blubber_measure_c,
                    'circumference',
                    NEW.whale_circumference,
                    'fetus_length',
                    NEW.whale_fetus_length,
                    'gender_id',
                    NEW.whale_gender_id,
                    'individual_number',
                    NEW.whale_individual_number,
                    'length',
                    NEW.whale_length
                );
        END IF;

        RETURN NEW;
    END;
$$;
