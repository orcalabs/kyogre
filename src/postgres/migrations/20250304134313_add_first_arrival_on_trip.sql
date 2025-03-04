ALTER TABLE trips
ADD COLUMN first_arrival timestamptz;

ALTER TABLE trips_detailed
ADD COLUMN first_arrival timestamptz;

UPDATE trips
SET
    first_arrival = UPPER(period)
WHERE
    trip_assembler_id = 2;

UPDATE trips_detailed
SET
    first_arrival = stop_timestamp
WHERE
    trip_assembler_id = 2;

ALTER TABLE trips
ADD CONSTRAINT check_first_arrival_is_correct CHECK (
    (
        trip_assembler_id != 2
        AND first_arrival IS NULL
    )
    OR (
        first_arrival IS NOT NULL
        AND first_arrival <= UPPER(period)
        AND first_arrival >= LOWER(period)
    )
);

ALTER TABLE trips_detailed
ADD CONSTRAINT check_first_arrival_is_correct CHECK (
    (
        trip_assembler_id != 2
        AND first_arrival IS NULL
    )
    OR (
        first_arrival IS NOT NULL
        AND first_arrival <= stop_timestamp
        AND first_arrival >= start_timestamp
    )
);

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
WHERE
    trip_assembler_id = 2;
