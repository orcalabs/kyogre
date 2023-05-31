DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'ers_dca_%';

DELETE FROM ers_dca;

DELETE FROM engine_transitions;

CREATE
OR REPLACE FUNCTION SUM_LIVING_WEIGHT (catches JSONB) RETURNS BIGINT LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN (
            SELECT
                COALESCE(SUM(c['living_weight']::BIGINT), 0)
            FROM
                JSONB_ARRAY_ELEMENTS(catches) c
        );
    END;
$$;

CREATE
OR REPLACE FUNCTION ARRAY_AGG_FROM_JSONB (items JSONB, idx TEXT) RETURNS JSONB[] LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN (
            SELECT
                COALESCE(ARRAY_AGG(DISTINCT i[idx]), '{}')
            FROM
                JSONB_ARRAY_ELEMENTS(items) i
        );
    END;
$$;

CREATE
OR REPLACE FUNCTION TO_VESSEL_LENGTH_GROUP (vessel_length DECIMAL) RETURNS INT LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN
            CASE
                WHEN vessel_length < 11 THEN 1
                WHEN vessel_length <@ numrange(11, 15, '[)') THEN 2
                WHEN vessel_length <@ numrange(15, 21, '[)') THEN 3
                WHEN vessel_length <@ numrange(21, 28, '[)') THEN 4
            ELSE 5
        END;
    END;
$$;

CREATE TABLE
    hauls (
        haul_id BIGSERIAL PRIMARY KEY,
        message_id BIGINT NOT NULL,
        start_timestamp TIMESTAMPTZ NOT NULL,
        stop_timestamp TIMESTAMPTZ NOT NULL,
        "period" TSTZRANGE NOT NULL GENERATED ALWAYS AS (TSTZRANGE (start_timestamp, stop_timestamp, '[]')) STORED,
        ers_activity_id TEXT NOT NULL REFERENCES ers_activities (ers_activity_id),
        duration INT NOT NULL,
        haul_distance INT,
        ocean_depth_end INT NOT NULL,
        ocean_depth_start INT NOT NULL,
        quota_type_id INT NOT NULL REFERENCES quota_types (quota_type_id),
        start_latitude DECIMAL NOT NULL,
        start_longitude DECIMAL NOT NULL,
        stop_latitude DECIMAL NOT NULL,
        stop_longitude DECIMAL NOT NULL,
        fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_call_sign TEXT,
        vessel_call_sign_ers TEXT NOT NULL,
        vessel_name TEXT,
        vessel_name_ers TEXT,
        vessel_length DECIMAL NOT NULL,
        vessel_length_group INT NOT NULL GENERATED ALWAYS AS (TO_VESSEL_LENGTH_GROUP (vessel_length)) STORED,
        catch_location_start TEXT REFERENCES catch_locations (catch_location_id),
        gear_id INT NOT NULL REFERENCES gear (gear_id),
        gear_group_id INT NOT NULL REFERENCES gear_groups (gear_group_id),
        total_living_weight BIGINT NOT NULL GENERATED ALWAYS AS (SUM_LIVING_WEIGHT (catches)) STORED,
        species_fao_ids TEXT[] NOT NULL GENERATED ALWAYS AS (
            ARRAY_AGG_FROM_JSONB (catches, 'species_fao_id')::TEXT[]
        ) STORED,
        species_fiskeridir_ids INT[] NOT NULL GENERATED ALWAYS AS (
            ARRAY_AGG_FROM_JSONB (catches, 'species_fiskeridir_id')::INT[]
        ) STORED,
        species_group_ids INT[] NOT NULL GENERATED ALWAYS AS (
            ARRAY_AGG_FROM_JSONB (catches, 'species_group_id')::INT[]
        ) STORED,
        species_main_group_ids INT[] NOT NULL GENERATED ALWAYS AS (
            ARRAY_AGG_FROM_JSONB (catches, 'species_main_group_id')::INT[]
        ) STORED,
        catches JSONB NOT NULL DEFAULT ('[]'),
        whale_catches JSONB NOT NULL DEFAULT ('[]'),
        FOREIGN KEY (message_id, start_timestamp, stop_timestamp) REFERENCES ers_dca (message_id, start_timestamp, stop_timestamp) ON DELETE CASCADE,
        UNIQUE (message_id, start_timestamp, stop_timestamp),
        CHECK (
            JSONB_ARRAY_LENGTH(catches) > 0
            OR JSONB_ARRAY_LENGTH(whale_catches) > 0
        )
    );

CREATE INDEX ON hauls (catch_location_start);

CREATE INDEX ON hauls (gear_group_id);

CREATE INDEX ON hauls (fiskeridir_vessel_id);

CREATE INDEX ON hauls (vessel_length_group);

CREATE INDEX ON hauls USING GIST (vessel_length);

CREATE INDEX ON hauls USING GIN (species_group_ids);

CREATE INDEX ON hauls USING GIST ("period");

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
                            ARRAY_AGG(c)
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
                            ARRAY_AGG(w)
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
    END;
$$;

CREATE TRIGGER ers_dca_catches_after_insert_add_catch_to_haul
AFTER INSERT ON ers_dca_catches FOR EACH ROW
EXECUTE FUNCTION add_catch_to_haul ();

CREATE TRIGGER ers_dca_whale_catches_after_insert_add_whale_catch_to_haul
AFTER INSERT ON ers_dca_whale_catches FOR EACH ROW
EXECUTE FUNCTION add_whale_catch_to_haul ();

CREATE TRIGGER ers_dca_catches_after_delete_remove_catch_from_haul
AFTER DELETE ON ers_dca_catches FOR EACH ROW
EXECUTE FUNCTION remove_catch_from_haul ();

CREATE TRIGGER ers_dca_whale_catches_after_delete_remove_whale_catch_from_haul
AFTER DELETE ON ers_dca_whale_catches FOR EACH ROW
EXECUTE FUNCTION remove_whale_catch_from_haul ();

CREATE
OR REPLACE FUNCTION update_database_views () RETURNS void LANGUAGE plpgsql AS $$
    BEGIN
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY trips_view';
    END
$$;

DROP MATERIALIZED VIEW trips_view;

CREATE MATERIALIZED VIEW
    public.trips_view TABLESPACE pg_default AS
SELECT
    q.trip_id,
    q.fiskeridir_vessel_id,
    q.period,
    q.period_precision,
    q.landing_coverage,
    q.trip_assembler_id,
    q.start_port_id,
    q.end_port_id,
    COALESCE(q.num_deliveries, 0::BIGINT) AS num_deliveries,
    COALESCE(q.total_gross_weight, 0::NUMERIC) AS total_gross_weight,
    COALESCE(q.total_living_weight, 0::NUMERIC) AS total_living_weight,
    COALESCE(q.total_product_weight, 0::NUMERIC) AS total_product_weight,
    COALESCE(q.delivery_points, '{}'::CHARACTER VARYING[]) AS delivery_points,
    COALESCE(q.gear_group_ids, '{}'::INTEGER[]) AS gear_group_ids,
    COALESCE(q.gear_main_group_ids, '{}'::INTEGER[]) AS gear_main_group_ids,
    COALESCE(q.gear_ids, '{}'::INTEGER[]) AS gear_ids,
    COALESCE(q.species_ids, '{}'::INTEGER[]) AS species_ids,
    COALESCE(q.species_fiskeridir_ids, '{}'::INTEGER[]) AS species_fiskeridir_ids,
    COALESCE(q.species_group_ids, '{}'::INTEGER[]) AS species_group_ids,
    COALESCE(q.species_main_group_ids, '{}'::INTEGER[]) AS species_main_group_ids,
    COALESCE(q.species_fao_ids, '{}'::CHARACTER VARYING[]) AS species_fao_ids,
    q.latest_landing_timestamp,
    COALESCE(q2.catches, '[]'::jsonb) AS catches,
    COALESCE(q3.hauls, '[]'::jsonb) AS hauls,
    COALESCE(q3.haul_ids, '{}'::BIGINT[]) AS haul_ids,
    COALESCE(q4.delivery_point_catches, '[]'::jsonb) AS delivery_point_catches,
    COALESCE(q5.vessel_events, '[]'::jsonb) AS vessel_events
FROM
    (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id,
            t.period,
            t.period_precision,
            t.landing_coverage,
            t.trip_assembler_id,
            t.start_port_id,
            t.end_port_id,
            COALESCE(COUNT(DISTINCT l.landing_id), 0::BIGINT) AS num_deliveries,
            COALESCE(SUM(le.living_weight), 0::NUMERIC) AS total_living_weight,
            COALESCE(SUM(le.gross_weight), 0::NUMERIC) AS total_gross_weight,
            COALESCE(SUM(le.product_weight), 0::NUMERIC) AS total_product_weight,
            ARRAY_AGG(DISTINCT l.delivery_point_id) FILTER (
                WHERE
                    l.delivery_point_id IS NOT NULL
            ) AS delivery_points,
            ARRAY_AGG(DISTINCT l.gear_main_group_id) FILTER (
                WHERE
                    l.gear_main_group_id IS NOT NULL
            ) AS gear_main_group_ids,
            ARRAY_AGG(DISTINCT l.gear_group_id) FILTER (
                WHERE
                    l.gear_group_id IS NOT NULL
            ) AS gear_group_ids,
            ARRAY_AGG(DISTINCT l.gear_id) FILTER (
                WHERE
                    l.gear_id IS NOT NULL
            ) AS gear_ids,
            ARRAY_AGG(DISTINCT le.species_id) FILTER (
                WHERE
                    le.species_id IS NOT NULL
            ) AS species_ids,
            ARRAY_AGG(DISTINCT le.species_fiskeridir_id) FILTER (
                WHERE
                    le.species_fiskeridir_id IS NOT NULL
            ) AS species_fiskeridir_ids,
            ARRAY_AGG(DISTINCT le.species_group_id) FILTER (
                WHERE
                    le.species_group_id IS NOT NULL
            ) AS species_group_ids,
            ARRAY_AGG(DISTINCT le.species_main_group_id) FILTER (
                WHERE
                    le.species_main_group_id IS NOT NULL
            ) AS species_main_group_ids,
            ARRAY_AGG(DISTINCT le.species_fao_id) FILTER (
                WHERE
                    le.species_fao_id IS NOT NULL
            ) AS species_fao_ids,
            MAX(l.landing_timestamp) AS latest_landing_timestamp
        FROM
            trips t
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
        GROUP BY
            t.trip_id
    ) q
    LEFT JOIN (
        SELECT
            qi.trip_id,
            COALESCE(JSONB_AGG(qi.catches), '[]'::jsonb) AS catches
        FROM
            (
                SELECT
                    t.trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        SUM(le.living_weight),
                        'gross_weight',
                        SUM(le.gross_weight),
                        'product_weight',
                        SUM(le.product_weight),
                        'species_fiskeridir_id',
                        le.species_fiskeridir_id,
                        'product_quality_id',
                        l.product_quality_id
                    ) AS catches
                FROM
                    trips t
                    JOIN vessel_events v ON t.trip_id = v.trip_id
                    JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                    JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
                GROUP BY
                    t.trip_id,
                    l.product_quality_id,
                    le.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) q2 ON q.trip_id = q2.trip_id
    LEFT JOIN (
        SELECT
            qi3.trip_id,
            ARRAY_AGG(DISTINCT qi3.haul_id) AS haul_ids,
            COALESCE(JSONB_AGG(qi3.hauls), '[]'::jsonb) AS hauls
        FROM
            (
                SELECT
                    t.trip_id,
                    h.haul_id,
                    JSONB_BUILD_OBJECT(
                        'haul_id',
                        h.haul_id,
                        'ers_activity_id',
                        h.ers_activity_id,
                        'duration',
                        h.duration,
                        'haul_distance',
                        h.haul_distance,
                        'catch_location_start',
                        h.catch_location_start,
                        'ocean_depth_end',
                        h.ocean_depth_end,
                        'ocean_depth_start',
                        h.ocean_depth_start,
                        'quota_type_id',
                        h.quota_type_id,
                        'start_latitude',
                        h.start_latitude,
                        'start_longitude',
                        h.start_longitude,
                        'start_timestamp',
                        LOWER(h.period),
                        'stop_timestamp',
                        UPPER(h.period),
                        'stop_latitude',
                        h.stop_latitude,
                        'stop_longitude',
                        h.stop_longitude,
                        'gear_group_id',
                        h.gear_group_id,
                        'gear_id',
                        h.gear_id,
                        'fiskeridir_vessel_id',
                        h.fiskeridir_vessel_id,
                        'vessel_call_sign',
                        h.vessel_call_sign,
                        'vessel_call_sign_ers',
                        h.vessel_call_sign_ers,
                        'vessel_length',
                        h.vessel_length,
                        'vessel_length_group',
                        h.vessel_length_group,
                        'vessel_name',
                        h.vessel_name,
                        'vessel_name_ers',
                        h.vessel_name_ers,
                        'catches',
                        COALESCE((ARRAY_AGG(h.catches)) [1], '[]'::jsonb),
                        'whale_catches',
                        COALESCE((ARRAY_AGG(h.whale_catches)) [1], '[]'::jsonb)
                    ) AS hauls
                FROM
                    trips t
                    JOIN hauls h ON h.period <@ t.period
                    AND t.fiskeridir_vessel_id = h.fiskeridir_vessel_id
                GROUP BY
                    t.trip_id,
                    h.haul_id,
                    h.ers_activity_id,
                    h.duration,
                    h.haul_distance,
                    h.catch_location_start,
                    h.ocean_depth_end,
                    h.ocean_depth_start,
                    h.quota_type_id,
                    h.start_latitude,
                    h.start_longitude,
                    h.period,
                    h.stop_latitude,
                    h.stop_longitude,
                    h.gear_group_id,
                    h.gear_id,
                    h.fiskeridir_vessel_id,
                    h.vessel_call_sign,
                    h.vessel_call_sign_ers,
                    h.vessel_length,
                    h.vessel_length_group,
                    h.vessel_name,
                    h.vessel_name_ers
                ORDER BY
                    (LOWER(h.period))
            ) qi3
        GROUP BY
            qi3.trip_id
    ) q3 ON q.trip_id = q3.trip_id
    LEFT JOIN (
        SELECT
            qi4.trip_id,
            COALESCE(
                JSONB_AGG(qi4.delivery_point_catches),
                '[]'::jsonb
            ) AS delivery_point_catches
        FROM
            (
                SELECT
                    qi42.trip_id,
                    JSONB_BUILD_OBJECT(
                        'delivery_point_id',
                        qi42.delivery_point_id,
                        'total_living_weight',
                        COALESCE(SUM(qi42.living_weight), 0::NUMERIC),
                        'total_gross_weight',
                        COALESCE(SUM(qi42.gross_weight), 0::NUMERIC),
                        'total_product_weight',
                        COALESCE(SUM(qi42.product_weight), 0::NUMERIC),
                        'catches',
                        COALESCE(JSONB_AGG(qi42.catches), '[]'::jsonb)
                    ) AS delivery_point_catches
                FROM
                    (
                        SELECT
                            t.trip_id,
                            l.delivery_point_id,
                            COALESCE(SUM(le.living_weight), 0::NUMERIC) AS living_weight,
                            COALESCE(SUM(le.product_weight), 0::NUMERIC) AS product_weight,
                            COALESCE(SUM(le.gross_weight), 0::NUMERIC) AS gross_weight,
                            JSONB_BUILD_OBJECT(
                                'living_weight',
                                COALESCE(SUM(le.living_weight), 0::NUMERIC),
                                'gross_weight',
                                COALESCE(SUM(le.gross_weight), 0::NUMERIC),
                                'product_weight',
                                COALESCE(SUM(le.product_weight), 0::NUMERIC),
                                'species_fiskeridir_id',
                                COALESCE(le.species_fiskeridir_id, 0),
                                'product_quality_id',
                                l.product_quality_id
                            ) AS catches
                        FROM
                            trips t
                            JOIN vessel_events v ON t.trip_id = v.trip_id
                            JOIN landings l ON l.vessel_event_id = v.vessel_event_id
                            JOIN landing_entries le ON l.landing_id::TEXT = le.landing_id::TEXT
                        WHERE
                            l.delivery_point_id IS NOT NULL
                        GROUP BY
                            t.trip_id,
                            l.delivery_point_id,
                            l.product_quality_id,
                            le.species_fiskeridir_id
                    ) qi42
                GROUP BY
                    qi42.trip_id,
                    qi42.delivery_point_id
            ) qi4
        GROUP BY
            qi4.trip_id
    ) q4 ON q.trip_id = q4.trip_id
    LEFT JOIN (
        SELECT
            t.trip_id,
            JSONB_AGG(
                JSONB_BUILD_OBJECT(
                    'vessel_event_id',
                    v.vessel_event_id,
                    'fiskeridir_vessel_id',
                    v.fiskeridir_vessel_id,
                    'timestamp',
                    v."timestamp",
                    'vessel_event_type_id',
                    v.vessel_event_type_id
                )
                ORDER BY
                    v."timestamp"
            ) AS vessel_events
        FROM
            trips t
            INNER JOIN vessel_events v ON v.trip_id = t.trip_id
        GROUP BY
            t.trip_id
    ) q5 ON q.trip_id = q5.trip_id
WITH
    DATA;

CREATE INDEX trips_view_haul_ids_idx ON public.trips_view USING gin (haul_ids);

CREATE INDEX trips_view_period_idx ON public.trips_view USING btree (period);

CREATE UNIQUE INDEX trips_view_trip_id_idx ON public.trips_view USING btree (trip_id);

CREATE INDEX ON trips_view (fiskeridir_vessel_id);

DROP MATERIALIZED VIEW hauls_view;
