CREATE INDEX ON trips (
    fiskeridir_vessel_id,
    LOWER(landing_coverage),
    UPPER(landing_coverage) DESC
);

CREATE INDEX ON trips (
    fiskeridir_vessel_id,
    LOWER(period),
    UPPER(period) DESC
);

CREATE INDEX ON vessel_events (
    fiskeridir_vessel_id,
    "timestamp",
    vessel_event_type_id
);

CREATE
OR REPLACE FUNCTION connect_trip_to_events () RETURNS TRIGGER LANGUAGE plpgsql AS $function$
    DECLARE _trip_id BIGINT;
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            IF (NEW.vessel_event_type_id = 1) THEN
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
                            AND NEW."timestamp" >= LOWER(landing_coverage)
                        )
                        OR (
                            NOT LOWER_INC(landing_coverage)
                            AND NEW."timestamp" > LOWER(landing_coverage)
                        )
                    )
                    AND (
                        (
                            UPPER_INC(landing_coverage)
                            AND NEW."timestamp" <= UPPER(landing_coverage)
                        )
                        OR (
                            NOT UPPER_INC(landing_coverage)
                            AND NEW."timestamp" < UPPER(landing_coverage)
                        )
                    );
            ELSIF (NEW.vessel_event_type_id = 3 OR NEW.vessel_event_type_id = 4) THEN
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
                            AND NEW."timestamp" >= LOWER(period)
                        )
                        OR (
                            NOT LOWER_INC(period)
                            AND NEW."timestamp"> LOWER(period)
                        )
                    )
                    AND (
                        (
                            UPPER_INC(period)
                            AND NEW."timestamp" <= UPPER(period)
                        )
                        OR (
                            NOT UPPER_INC(period)
                            AND NEW."timestamp" < UPPER(period)
                        )
                    );
            ELSE
                SELECT
                    trip_id INTO _trip_id
                FROM
                    trips
                WHERE
                    fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                    AND (
                        (
                            LOWER_INC(period)
                            AND NEW."timestamp" >= LOWER(period)
                        )
                        OR (
                            NOT LOWER_INC(period)
                            AND NEW."timestamp"> LOWER(period)
                        )
                    )
                    AND (
                        (
                            UPPER_INC(period)
                            AND NEW."timestamp" <= UPPER(period)
                        )
                        OR (
                            NOT UPPER_INC(period)
                            AND NEW."timestamp" < UPPER(period)
                        )
                    );
            END IF;
            NEW.trip_id = _trip_id;
        END IF;
        RETURN NEW;
    END
$function$;
