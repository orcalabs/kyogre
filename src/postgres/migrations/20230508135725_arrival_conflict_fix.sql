CREATE
OR REPLACE FUNCTION public.add_conflicting_ers_arrival () RETURNS TRIGGER LANGUAGE plpgsql AS $$
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
                    AND NOT EXISTS (
                        SELECT
                            departure_timestamp
                        FROM
                            ers_departures
                        WHERE
                            fiskeridir_vessel_id = NEW.fiskeridir_vessel_id
                            AND departure_timestamp > tt.timer
                            AND NEW.arrival_timestamp > departure_timestamp
                        ORDER BY
                            departure_timestamp DESC
                        LIMIT
                            1
                    )
                )
                OR NEW.arrival_timestamp < tt.timer
            ON CONFLICT (fiskeridir_vessel_id) DO
            UPDATE
            SET
                "conflict" = excluded."conflict"
            WHERE
                trip_assembler_conflicts."conflict" > excluded."conflict";
        END IF;
        RETURN NULL;
   END;
$$;
