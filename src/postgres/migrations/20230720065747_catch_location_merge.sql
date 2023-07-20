UPDATE hauls h
SET
    catch_locations = (
        SELECT
            ARRAY_AGG(DISTINCT e) FILTER (
                WHERE
                    e IS NOT NULL
            )
        FROM
            UNNEST(h.catch_locations || h.catch_location_start) e
    );

ALTER TABLE hauls
ADD CONSTRAINT non_empty_catch_locations CHECK (
    (
        catch_locations IS NULL
        AND catch_location_start IS NULL
    )
    OR catch_location_start = ANY (catch_locations)
);

CREATE
OR REPLACE FUNCTION empty_array_to_null (val VARCHAR[]) RETURNS VARCHAR[] LANGUAGE PLPGSQL AS $$
begin
    IF array_length(val, 1) IS NULL THEN
        RETURN NULL;
    ELSE
        RETURN val;
    END IF;
end;
$$;

CREATE
OR REPLACE FUNCTION add_catch_to_haul () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
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
                    catch_locations,
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
                empty_array_to_null(ARRAY_REMOVE(ARRAY[l.catch_location_id], NULL)),
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
$function$;
