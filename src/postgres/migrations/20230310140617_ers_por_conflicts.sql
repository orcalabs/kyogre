CREATE
OR REPLACE FUNCTION add_conflicting_ers_arrival () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trip_assembler_conflicts (
                    fiskeridir_vessel_id,
                    "conflict",
                    trip_assembler_id
                )
            SELECT
                NEW.fiskeridir_vessel_id,
                NEW.arrival_timestamp,
                t.trip_assembler_id
            FROM
                trip_assemblers AS t
                INNER JOIN trip_calculation_timers AS tt ON NEW.fiskeridir_vessel_id = tt.fiskeridir_vessel_id
                AND t.trip_assembler_id = tt.trip_assembler_id
            WHERE
                t.trip_assembler_id IN (
                    SELECT
                        trip_assembler_id
                    FROM
                        trip_assembler_data_sources
                    WHERE
                        trip_assembler_data_source_id = 'ers'
                )
                AND (
                    NEW.arrival_timestamp > tt.timer
                    AND EXISTS (
                        SELECT
                            departure_timestamp
                        FROM
                            ers_departures
                        WHERE
                            fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                            AND departure_timestamp < tt.timer
                            AND NEW.arrival_timestamp > departure_timestamp
                        ORDER BY
                            departure_timestamp DESC
                        LIMIT
                            1
                    )
                )
                OR NEW.arrival_timestamp < tt.timer
            ON CONFLICT (fiskeridir_vessel_id, trip_assembler_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$function$;

CREATE
OR REPLACE FUNCTION public.set_landing_coverage () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    declare _next_start timestamptz;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF NEW.trip_assembler_id = 1 THEN
                NEW.landing_coverage = NEW.period;
            ELSIF NEW.trip_assembler_id = 2 THEN
                UPDATE trips AS t
                SET
                    landing_coverage = tstzrange (LOWER(t.period), LOWER(NEW.period))
                FROM
                    (
                        SELECT
                            trip_id
                        FROM
                            trips
                        WHERE
                            trip_assembler_id = 2
                            AND period < NEW.period
                        ORDER BY
                            period DESC
                        LIMIT
                            1
                    ) AS prior_trip
                WHERE
                    t.trip_id = prior_trip.trip_id;

                SELECT
                    COALESCE(
                        (
                            SELECT
                                LOWER(period) INTO _next_start
                            FROM
                                trips
                            WHERE
                                period > NEW.period
                            ORDER BY
                                period ASC
                            LIMIT
                                1
                        ),
                        timestamptz ('2200-01-01 00:00:00-00')
                    );
                NEW.landing_coverage = tstzrange(lower(NEW.period), _next_start);
            END IF;
        END IF;
        RETURN NEW;
    END;
$function$;

CREATE
OR REPLACE FUNCTION add_conflicting_ers_departure () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trip_assembler_conflicts (
                    fiskeridir_vessel_id,
                    "conflict",
                    trip_assembler_id
                )
            SELECT
                NEW.fiskeridir_vessel_id,
                NEW.departure_timestamp,
                t.trip_assembler_id
            FROM
                trip_assemblers AS t
                INNER JOIN trip_calculation_timers AS tt ON NEW.fiskeridir_vessel_id = tt.fiskeridir_vessel_id
                AND t.trip_assembler_id = tt.trip_assembler_id
            WHERE
                t.trip_assembler_id IN (
                    SELECT
                        trip_assembler_id
                    FROM
                        trip_assembler_data_sources
                    WHERE
                        trip_assembler_data_source_id = 'ers'
                )
                AND NEW.departure_timestamp < tt.timer
            ON CONFLICT (fiskeridir_vessel_id, trip_assembler_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$function$;

CREATE TRIGGER ers_arrivals_after_insert_add_conflict
AFTER INSERT ON ers_arrivals FOR EACH ROW
EXECUTE FUNCTION add_conflicting_ers_arrival ();

CREATE TRIGGER ers_departure_after_insert_add_conflict
AFTER INSERT ON ers_departures FOR EACH ROW
EXECUTE FUNCTION add_conflicting_ers_departure ();

CREATE INDEX ON ers_departures (departure_timestamp);

CREATE INDEX ON ers_departures (fiskeridir_vessel_id);

CREATE INDEX ON ers_arrivals (arrival_timestamp);

CREATE INDEX ON ers_arrivals (fiskeridir_vessel_id);
