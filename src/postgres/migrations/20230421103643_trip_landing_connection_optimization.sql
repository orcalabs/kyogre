CREATE
OR REPLACE FUNCTION public.connect_trip_to_landings () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            INSERT INTO
                trips__landings (trip_id, landing_id, trip_assembler_id)
            SELECT
                NEW.trip_id,
                landing_id,
                NEW.trip_assembler_id
            FROM
                landings AS l
            WHERE
                l.fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                AND (
                    l.landing_timestamp > LOWER(NEW.landing_coverage)
                    AND l.landing_timestamp <= UPPER(NEW.landing_coverage)
                )
            ON CONFLICT (landing_id, trip_assembler_id) DO
            UPDATE
            SET
                trip_id = NEW.trip_id;
        END IF;
        RETURN NULL;
    END;
$function$;

CREATE INDEX ON landings (fiskeridir_vessel_id, landing_timestamp);
