TRUNCATE ers_tra CASCADE;

DELETE FROM data_hashes
WHERE
    data_hash_id LIKE 'ers_tra%';

ALTER TABLE vessel_events
ADD COLUMN occurence_timestamp timestamptz;

ALTER TABLE vessel_events
RENAME COLUMN "timestamp" TO report_timestamp;

CREATE
OR REPLACE FUNCTION SUM_WEIGHT (source JSONB, field TEXT) RETURNS DECIMAL LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN (
            SELECT
                COALESCE(SUM(c[field]::DECIMAL), 0)
            FROM
                JSONB_ARRAY_ELEMENTS(source) c
        );
    END;
$$;

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

ALTER TABLE ers_tra
DROP CONSTRAINT ers_tra_check;

ALTER TABLE ers_tra
ADD CONSTRAINT ers_tra_check CHECK (
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

ALTER TABLE trip_calculation_timers
ADD COLUMN queued_reset BOOLEAN DEFAULT FALSE;

CREATE TABLE
    trips_refresh_boundary (
        onerow_id bool PRIMARY KEY DEFAULT TRUE,
        refresh_boundary timestamptz,
        CONSTRAINT onerow_uni CHECK (onerow_id)
    );

INSERT INTO
    trips_refresh_boundary (refresh_boundary)
VALUES
    (NULL);

CREATE INDEX ON trips (LOWER("period"));

CREATE TABLE
    trips_detailed2 (
        trip_id BIGINT PRIMARY KEY REFERENCES trips (trip_id) ON DELETE CASCADE,
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        fiskeridir_length_group_id INT NOT NULL REFERENCES fiskeridir_length_groups (fiskeridir_length_group_id),
        "period" tstzrange NOT NULL,
        landing_coverage tstzrange NOT NULL,
        period_precision tstzrange,
        start_timestamp timestamptz NOT NULL GENERATED ALWAYS AS (LOWER("period")) STORED,
        stop_timestamp timestamptz NOT NULL GENERATED ALWAYS AS (UPPER("period")) STORED,
        trip_assembler_id INT NOT NULL REFERENCES trip_assemblers (trip_assembler_id),
        most_recent_landing timestamptz,
        start_port_id VARCHAR REFERENCES ports (port_id),
        end_port_id VARCHAR REFERENCES ports (port_id),
        delivery_point_ids VARCHAR[],
        num_landings INT GENERATED ALWAYS AS (CARDINALITY(landing_ids)) STORED,
        landing_gear_ids INT[],
        landing_gear_group_ids INT[],
        landing_species_group_ids INT[],
        vessel_events jsonb,
        fishing_facilities jsonb,
        haul_ids BIGINT[] GENERATED ALWAYS AS (ARRAY_AGG_FROM_JSONB (hauls, 'haul_id')::BIGINT[]) STORED,
        landings jsonb,
        landing_ids VARCHAR[],
        landing_total_living_weight DECIMAL GENERATED ALWAYS AS (SUM_WEIGHT (landings, 'living_weight')::DECIMAL) STORED,
        landing_total_product_weight DECIMAL GENERATED ALWAYS AS (SUM_WEIGHT (landings, 'product_weight')::DECIMAL) STORED,
        landing_total_gross_weight DECIMAL GENERATED ALWAYS AS (SUM_WEIGHT (landings, 'gross_weight')::DECIMAL) STORED,
        hauls jsonb
    );

INSERT INTO
    trips_detailed2 (
        trip_id,
        fiskeridir_vessel_id,
        fiskeridir_length_group_id,
        "period",
        landing_coverage,
        period_precision,
        trip_assembler_id,
        most_recent_landing,
        start_port_id,
        end_port_id,
        delivery_point_ids,
        landing_gear_ids,
        landing_gear_group_ids,
        landing_species_group_ids,
        vessel_events,
        fishing_facilities,
        landings,
        landing_ids,
        hauls
    )
SELECT
    trip_id,
    fiskeridir_vessel_id,
    fiskeridir_length_group_id,
    "period",
    landing_coverage,
    period_precision,
    trip_assembler_id,
    most_recent_landing,
    start_port_id,
    end_port_id,
    delivery_point_ids,
    landing_gear_ids,
    landing_gear_group_ids,
    landing_species_group_ids,
    vessel_events,
    fishing_facilities,
    landings,
    landing_ids,
    hauls
FROM
    trips_detailed;

DROP MATERIALIZED VIEW trips_detailed;

ALTER TABLE trips_detailed2
RENAME TO trips_detailed;

CREATE INDEX ON trips_detailed USING gin (delivery_point_ids);

CREATE INDEX ON trips_detailed USING gin (landing_gear_ids);

CREATE INDEX ON trips_detailed USING gin (landing_gear_group_ids);

CREATE INDEX ON trips_detailed USING gin (landing_species_group_ids);

CREATE INDEX ON trips_detailed USING gin (landing_ids);

CREATE INDEX ON trips_detailed USING gin (haul_ids);

CREATE INDEX ON trips_detailed USING gist ("period");

CREATE INDEX ON trips_detailed (fiskeridir_vessel_id);

CREATE INDEX ON trips_detailed (fiskeridir_length_group_id);

CREATE INDEX ON trips_detailed (start_timestamp, stop_timestamp);

CREATE INDEX ON trips_detailed (stop_timestamp);

CREATE INDEX ON trips_detailed (landing_total_living_weight);
