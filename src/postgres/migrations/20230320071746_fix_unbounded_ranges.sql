DELETE FROM trip_calculation_timers;

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
                    landing_coverage = tstzrange (LOWER(t.period), LOWER(NEW.period), '(]')
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
                NEW.landing_coverage = tstzrange(lower(NEW.period), _next_start, '(]');
            END IF;
        END IF;
        RETURN NEW;
    END;
$function$;
