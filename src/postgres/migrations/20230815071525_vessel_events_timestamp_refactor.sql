ALTER TABLE vessel_events
ADD COLUMN occurence_timestamp timestamptz;

ALTER TABLE vessel_events
RENAME COLUMN "timestamp" TO report_timestamp;

ALTER TABLE ers_tra
DROP CONSTRAINT ers_tra_check;

ALTER TABLE ers_tra
ADD CONSTRAINT ers_dca_check CHECK (
    (
        (
            (vessel_event_id IS NULL)
            AND (fiskeridir_vessel_id IS NULL)
        )
        OR (
            (vessel_event_id IS NOT NULL)
            AND (fiskeridir_vessel_id IS NOT NULL)
        )
    )
);

CREATE
OR REPLACE FUNCTION public.add_vessel_event (
    vessel_event_type_id INTEGER,
    fiskeridir_vessel_id BIGINT,
    occurence_timestamp TIMESTAMP WITH TIME ZONE,
    report_timestamp TIMESTAMP WITH TIME ZONE
) RETURNS BIGINT LANGUAGE plpgsql AS $function$
    DECLARE
        _event_id bigint;
    BEGIN
        IF fiskeridir_vessel_id IS NULL THEN
            RETURN NULL;
        ELSE
            INSERT INTO
                vessel_events (
                    vessel_event_type_id,
                    fiskeridir_vessel_id,
                    occurence_timestamp,
                    report_timestamp
                ) values (
                    vessel_event_type_id,
                    fiskeridir_vessel_id,
                    occurence_timestamp,
                    report_timestamp
                )
            RETURNING
                vessel_event_id into _event_id;
        END IF;
        RETURN _event_id;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.add_tra_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NOT EXISTS (
                SELECT
                    1
                FROM
                    ers_tra
                WHERE
                    message_id = NEW.message_id) THEN
                NEW.vessel_event_id = add_vessel_event(5, NEW.fiskeridir_vessel_id, NEW.reloading_timestamp, NEW.message_timestamp);
                RETURN NEW;
            ELSE
                RETURN NULL;
            END IF;
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.add_ers_departure_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NOT EXISTS (
                SELECT
                    1
                FROM
                    ers_departures
                WHERE
                    message_id = NEW.message_id) THEN
                NEW.vessel_event_id = add_vessel_event(4, NEW.fiskeridir_vessel_id, NEW.departure_timestamp, NEW.message_timestamp);
                RETURN NEW;
            ELSE
                RETURN NULL;
            END IF;
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.add_ers_arrival_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NOT EXISTS (
                SELECT
                    1
                FROM
                    ers_arrivals
                WHERE
                    message_id = NEW.message_id) THEN
                NEW.vessel_event_id = add_vessel_event(3, NEW.fiskeridir_vessel_id, NEW.arrival_timestamp, NEW.message_timestamp);
                RETURN NEW;
            ELSE
                RETURN NULL;
            END IF;
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.add_ers_dca_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            NEW.vessel_event_id = add_vessel_event(2, NEW.fiskeridir_vessel_id, NEW.start_timestamp, NEW.message_timestamp);
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.add_landing_vessel_event () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
           NEW.vessel_event_id = add_vessel_event(1, NEW.fiskeridir_vessel_id, NEW.landing_timestamp, NEW.landing_timestamp);
        END IF;
        RETURN NEW;
   END;
$function$;

UPDATE vessel_events e
SET
    report_timestamp = d.message_timestamp,
    occurence_timestamp = d.start_timestamp
FROM
    ers_dca d
WHERE
    d.vessel_event_id = e.vessel_event_id;

UPDATE vessel_events e
SET
    report_timestamp = d.message_timestamp,
    occurence_timestamp = d.arrival_timestamp
FROM
    ers_arrivals d
WHERE
    d.vessel_event_id = e.vessel_event_id;

UPDATE vessel_events e
SET
    report_timestamp = d.message_timestamp,
    occurence_timestamp = d.departure_timestamp
FROM
    ers_departures d
WHERE
    d.vessel_event_id = e.vessel_event_id;

INSERT INTO
    vessel_events (
        occurence_timestamp,
        report_timestamp,
        vessel_event_type_id,
        fiskeridir_vessel_id
    )
SELECT
    reloading_timestamp,
    message_timestamp,
    5,
    fiskeridir_vessel_id
FROM
    ers_tra d
WHERE
    d.reloading_timestamp IS NULL;

CREATE
OR REPLACE FUNCTION connect_events_to_trip () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            UPDATE vessel_events
            SET
                trip_id = NEW.trip_id
            WHERE
                (
                    (vessel_event_type_id = 5
                    OR vessel_event_type_id = 2)
                    AND COALESCE(occurence_timestamp, report_timestamp) <@ NEW.period
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                )
                OR (
                    vessel_event_type_id = 3
                    AND report_timestamp <= UPPER(NEW.period)
                    AND report_timestamp > LOWER(NEW.period)
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                )
                OR (
                    vessel_event_type_id = 4
                    AND report_timestamp < UPPER(NEW.period)
                    AND report_timestamp >= LOWER(NEW.period)
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                )
                OR (
                    vessel_event_type_id = 1
                    AND occurence_timestamp <@ NEW.landing_coverage
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                );
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE
OR REPLACE FUNCTION connect_trip_to_events () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    DECLARE _trip_id BIGINT;
    DECLARE _timestamp timestamptz;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.vessel_event_type_id = 1) THEN
                _timestamp = NEW.occurence_timestamp;
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND trip_assembler_id != 1
                    AND (
                        (
                            LOWER_INC(landing_coverage)
                            AND _timestamp >= LOWER(landing_coverage)
                        )
                        OR (
                            NOT LOWER_INC(landing_coverage)
                            AND _timestamp > LOWER(landing_coverage)
                        )
                    )
                    AND (
                        (
                            UPPER_INC(landing_coverage)
                            AND _timestamp <= UPPER(landing_coverage)
                        )
                        OR (
                            NOT UPPER_INC(landing_coverage)
                            AND _timestamp < UPPER(landing_coverage)
                        )
                    );
            ELSIF (NEW.vessel_event_type_id = 3 OR NEW.vessel_event_type_id = 4) THEN
                _timestamp = NEW.report_timestamp;
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND trip_assembler_id != 2
                    AND (
                        (
                            LOWER_INC(period)
                            AND _timestamp >= LOWER(period)
                        )
                        OR (
                            NOT LOWER_INC(period)
                            AND _timestamp > LOWER(period)
                        )
                    )
                    AND (
                        (
                            UPPER_INC(period)
                            AND _timestamp <= UPPER(period)
                        )
                        OR (
                            NOT UPPER_INC(period)
                            AND _timestamp < UPPER(period)
                        )
                    );
            ELSE
                _timestamp = COALESCE(NEW.occurence_timestamp, NEW.report_timestamp);
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND (
                        (
                            LOWER_INC(period)
                            AND _timestamp >= LOWER(period)
                        )
                        OR (
                            NOT LOWER_INC(period)
                            AND _timestamp > LOWER(period)
                        )
                    )
                    AND (
                        (
                            UPPER_INC(period)
                            AND _timestamp <= UPPER(period)
                        )
                        OR (
                            NOT UPPER_INC(period)
                            AND _timestamp < UPPER(period)
                        )
                    );
            END IF;
            NEW.trip_id = _trip_id;
        END IF;
        RETURN NEW;
    END
$function$;

CREATE
OR REPLACE FUNCTION public.update_trip_landing_event_connections () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF (NEW.landing_coverage != OLD.landing_coverage) THEN
                UPDATE vessel_events
                SET
                    trip_id = NULL
                WHERE
                    vessel_event_type_id = 1
                    AND occurence_timestamp <@ OLD.landing_coverage
                    AND fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
                UPDATE vessel_events
                SET
                    trip_id = NEW.trip_id
                WHERE
                    vessel_event_type_id = 1
                    AND occurence_timestamp <@ NEW.landing_coverage
                    AND fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            END IF;
        END IF;
        RETURN NEW;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.add_trip_assembler_conflict () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    DECLARE _fiskeridir_vessel_id BIGINT;
    DECLARE _event_timestamp timestamptz;
    DECLARE _event_type_id int;
    DECLARE _assembler_id int;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            _fiskeridir_vessel_id = NEW.fiskeridir_vessel_id;
            _event_timestamp = NEW.report_timestamp;
            _event_type_id = NEW.vessel_event_type_id;
        ELSIF (TG_OP = 'DELETE') THEN
            _fiskeridir_vessel_id = OLD.fiskeridir_vessel_id;
            _event_timestamp = OLD.report_timestamp;
            _event_type_id = OLD.vessel_event_type_id;
        ELSE
            RETURN NEW;
        END IF;

        IF (_event_type_id = 1) THEN
            _assembler_id = 1;
        ELSIF (_event_type_id = 3 OR _event_type_id = 4) THEN
            _assembler_id = 2;
        ELSE
            RETURN NEW;
        END IF;
        INSERT INTO
            trip_assembler_conflicts (
                fiskeridir_vessel_id,
                "conflict",
                trip_assembler_id
            )
        SELECT
            _fiskeridir_vessel_id,
            _event_timestamp,
            t.trip_assembler_id
        FROM
            trip_calculation_timers AS t
        WHERE
            t.fiskeridir_vessel_id = _fiskeridir_vessel_id
            AND t.trip_assembler_id = _assembler_id
            AND t.timer >= _event_timestamp
        ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
        SET
            "conflict" = excluded."conflict"
        WHERE
            trip_assembler_conflicts."conflict" > excluded."conflict";
        RETURN NEW;
    END
$function$;

DROP MATERIALIZED VIEW trips_detailed;

CREATE MATERIALIZED VIEW
    trips_detailed AS
WITH
    everything AS (
        SELECT
            t.trip_id,
            t.fiskeridir_vessel_id AS t_fiskeridir_vessel_id,
            t.period AS trip_period,
            UPPER(t.period) AS trip_stop_timestamp,
            LOWER(t.period) AS trip_start_timestamp,
            t.trip_assembler_id AS t_trip_assembler_id,
            t.period_precision,
            fv.fiskeridir_length_group_id,
            t.landing_coverage,
            t.trip_assembler_id,
            t.start_port_id,
            t.end_port_id,
            l.landing_timestamp,
            l.delivery_point_id,
            l.gear_id AS landing_gear_id,
            l.gear_group_id AS landing_gear_group_id,
            le.species_group_id AS landing_species_group_id,
            l.landing_id,
            le.living_weight,
            le.gross_weight,
            le.product_weight,
            l.product_quality_id,
            le.species_fiskeridir_id,
            v.vessel_event_id AS v_vessel_event_id,
            v.fiskeridir_vessel_id AS v_fiskeridir_vessel_id,
            v.report_timestamp,
            v.occurence_timestamp,
            v.vessel_event_type_id AS v_vessel_event_type_id,
            h.*,
            f.tool_id,
            f.barentswatch_vessel_id,
            f.fiskeridir_vessel_id AS f_fiskeridir_vessel_id,
            f.vessel_name AS f_vessel_name,
            f.call_sign AS f_call_sign,
            f.mmsi,
            f.imo,
            f.reg_num,
            f.sbr_reg_num,
            f.contact_phone,
            f.contact_email,
            f.tool_type,
            f.tool_type_name,
            f.tool_color,
            f.tool_count,
            f.setup_timestamp,
            f.setup_processed_timestamp,
            f.removed_timestamp,
            f.removed_processed_timestamp,
            f.last_changed,
            f.source,
            f.comment,
            ST_ASTEXT (f.geometry_wkt) AS geometry,
            f.api_source
        FROM
            trips t
            INNER JOIN fiskeridir_vessels fv ON fv.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            LEFT JOIN vessel_events v ON t.trip_id = v.trip_id
            LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
            LEFT JOIN landing_entries le ON l.landing_id = le.landing_id
            LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
            LEFT JOIN fishing_facilities f ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
            AND f.period && t.period
    )
SELECT
    e.trip_id,
    MAX(e.t_fiskeridir_vessel_id) AS fiskeridir_vessel_id,
    MAX(e.fiskeridir_length_group_id) AS fiskeridir_length_group_id,
    (ARRAY_AGG(e.trip_period)) [1] AS "period",
    (ARRAY_AGG(e.landing_coverage)) [1] AS landing_coverage,
    (ARRAY_AGG(e.period_precision)) [1] AS period_precision,
    MAX(e.trip_start_timestamp) AS start_timestamp,
    MAX(e.trip_stop_timestamp) AS stop_timestamp,
    MAX(e.t_trip_assembler_id) AS trip_assembler_id,
    MAX(e.landing_timestamp) AS most_recent_landing,
    MAX(e.start_port_id) AS start_port_id,
    MAX(e.end_port_id) AS end_port_id,
    ARRAY_AGG(DISTINCT e.delivery_point_id) FILTER (
        WHERE
            e.delivery_point_id IS NOT NULL
    ) AS delivery_point_ids,
    COUNT(DISTINCT e.landing_id) FILTER (
        WHERE
            e.landing_id IS NOT NULL
    ) AS num_landings,
    ARRAY_AGG(DISTINCT e.landing_gear_id) FILTER (
        WHERE
            e.landing_gear_id IS NOT NULL
    ) AS landing_gear_ids,
    ARRAY_AGG(DISTINCT e.landing_gear_group_id) FILTER (
        WHERE
            e.landing_gear_group_id IS NOT NULL
    ) AS landing_gear_group_ids,
    ARRAY_AGG(DISTINCT e.landing_species_group_id) FILTER (
        WHERE
            e.landing_species_group_id IS NOT NULL
    ) AS landing_species_group_ids,
    COALESCE(
        JSONB_AGG(
            DISTINCT JSONB_BUILD_OBJECT(
                'vessel_event_id',
                e.v_vessel_event_id,
                'fiskeridir_vessel_id',
                e.v_fiskeridir_vessel_id,
                'report_timestamp',
                e.report_timestamp,
                'occurence_timestamp',
                e.occurence_timestamp,
                'vessel_event_type_id',
                e.v_vessel_event_type_id
            )
        ) FILTER (
            WHERE
                e.v_vessel_event_id IS NOT NULL
        ),
        '[]'
    ) AS vessel_events,
    COALESCE(
        JSONB_AGG(
            DISTINCT JSONB_BUILD_OBJECT(
                'tool_id',
                e.tool_id,
                'barentswatch_vessel_id',
                e.barentswatch_vessel_id,
                'fiskeridir_vessel_id',
                e.f_fiskeridir_vessel_id,
                'vessel_name',
                e.f_vessel_name,
                'call_sign',
                e.f_call_sign,
                'mmsi',
                e.mmsi,
                'imo',
                e.imo,
                'reg_num',
                e.reg_num,
                'sbr_reg_num',
                e.sbr_reg_num,
                'contact_phone',
                e.contact_phone,
                'contact_email',
                e.contact_email,
                'tool_type',
                e.tool_type,
                'tool_type_name',
                e.tool_type_name,
                'tool_color',
                e.tool_color,
                'tool_count',
                e.tool_count,
                'setup_timestamp',
                e.setup_timestamp,
                'setup_processed_timestamp',
                e.setup_processed_timestamp,
                'removed_timestamp',
                e.removed_timestamp,
                'removed_processed_timestamp',
                e.removed_processed_timestamp,
                'last_changed',
                e.last_changed,
                'source',
                e.source,
                'comment',
                e.comment,
                'geometry_wkt',
                e.geometry,
                'api_source',
                e.api_source
            )
        ) FILTER (
            WHERE
                e.tool_id IS NOT NULL
        ),
        '[]'
    ) AS fishing_facilities,
    ARRAY_AGG(DISTINCT e.haul_id) FILTER (
        WHERE
            e.haul_id IS NOT NULL
    ) AS haul_ids,
    (
        ARRAY_AGG(DISTINCT landings.catches) FILTER (
            WHERE
                landings.catches IS NOT NULL
        )
    ) [1] AS landings,
    ARRAY_AGG(DISTINCT e.landing_id) FILTER (
        WHERE
            e.landing_id IS NOT NULL
    ) AS landing_ids,
    COALESCE(SUM(e.living_weight), 0) AS landing_total_living_weight,
    COALESCE(SUM(e.product_weight), 0) AS landing_total_product_weight,
    COALESCE(SUM(e.gross_weight), 0) AS landing_total_gross_weight,
    COALESCE(
        JSONB_AGG(
            DISTINCT JSONB_BUILD_OBJECT(
                'haul_id',
                e.haul_id,
                'ers_activity_id',
                e.ers_activity_id,
                'duration',
                e.duration,
                'haul_distance',
                e.haul_distance,
                'catch_location_start',
                e.catch_location_start,
                'catch_locations',
                e.catch_locations,
                'ocean_depth_end',
                e.ocean_depth_end,
                'ocean_depth_start',
                e.ocean_depth_start,
                'quota_type_id',
                e.quota_type_id,
                'start_latitude',
                e.start_latitude,
                'start_longitude',
                e.start_longitude,
                'start_timestamp',
                LOWER(e.period),
                'stop_timestamp',
                UPPER(e.period),
                'stop_latitude',
                e.stop_latitude,
                'stop_longitude',
                e.stop_longitude,
                'gear_group_id',
                e.gear_group_id,
                'gear_id',
                e.gear_id,
                'fiskeridir_vessel_id',
                e.fiskeridir_vessel_id,
                'vessel_call_sign',
                e.vessel_call_sign,
                'vessel_call_sign_ers',
                e.vessel_call_sign_ers,
                'vessel_length',
                e.vessel_length,
                'vessel_length_group',
                e.vessel_length_group,
                'vessel_name',
                e.vessel_name,
                'vessel_name_ers',
                e.vessel_name_ers,
                'total_living_weight',
                e.total_living_weight,
                'catches',
                e.catches,
                'whale_catches',
                e.whale_catches
            )
        ) FILTER (
            WHERE
                e.haul_id IS NOT NULL
        )
    ) AS hauls
FROM
    everything e
    LEFT JOIN (
        SELECT
            qi.trip_id,
            COALESCE(
                JSONB_AGG(qi.catches) FILTER (
                    WHERE
                        qi.catches IS NOT NULL
                ),
                '[]'
            ) AS catches
        FROM
            (
                SELECT
                    e.trip_id,
                    JSONB_BUILD_OBJECT(
                        'living_weight',
                        COALESCE(SUM(e.living_weight), 0),
                        'gross_weight',
                        COALESCE(SUM(e.gross_weight), 0),
                        'product_weight',
                        COALESCE(SUM(e.product_weight), 0),
                        'species_fiskeridir_id',
                        e.species_fiskeridir_id,
                        'product_quality_id',
                        e.product_quality_id
                    ) AS catches
                FROM
                    everything e
                WHERE
                    e.product_quality_id IS NOT NULL
                    AND e.species_fiskeridir_id IS NOT NULL
                GROUP BY
                    e.trip_id,
                    e.product_quality_id,
                    e.species_fiskeridir_id
            ) qi
        GROUP BY
            qi.trip_id
    ) landings ON e.trip_id = landings.trip_id
GROUP BY
    e.trip_id;

CREATE UNIQUE INDEX ON trips_detailed (trip_id);

CREATE INDEX ON trips_detailed USING gin (delivery_point_ids);

CREATE INDEX ON trips_detailed USING gin (landing_gear_ids);

CREATE INDEX ON trips_detailed USING gin (landing_gear_group_ids);

CREATE INDEX ON trips_detailed USING gin (landing_species_group_ids);

CREATE INDEX ON trips_detailed USING gin (haul_ids);

CREATE INDEX ON trips_detailed USING gin (landing_ids);

CREATE INDEX ON trips_detailed (landing_total_living_weight);

CREATE INDEX ON trips_detailed (start_timestamp);

CREATE INDEX ON trips_detailed (stop_timestamp);

CREATE INDEX ON trips_detailed (fiskeridir_vessel_id);

CREATE INDEX ON trips_detailed (start_timestamp, stop_timestamp);
