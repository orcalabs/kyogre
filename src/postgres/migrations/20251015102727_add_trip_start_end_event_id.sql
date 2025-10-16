-- We do not define foreign keys directly to the `vessel_event_types` table as this is done in the `vessel_events` table that we define a foreign key to further down in this migration.
ALTER TABLE trips
ADD start_vessel_event_id BIGINT UNIQUE,
ADD start_vessel_event_type INT,
ADD end_vessel_event_id BIGINT UNIQUE,
ADD end_vessel_event_type INT;

-- We need these to be nullable as Landing events can be deleted (when they are removed from fiskeridir csv files), if landings are deleted their trips will be re-assembled by the TripAssembler.
-- `start_vessel_event_id` can be null for the first ever landing of a vessel as we create an artifical landing event as the start of the first trip.
-- ERS por/dep are never deleted and we can therefore enforce that start/end event id is always set.
-- A check constraint to verify that start/end event id are set correctly will be added in the next migration when all trips have been re-generated with their appropriate start/end event ids.
ALTER TABLE trips
ADD CONSTRAINT start_vessel_event_fk FOREIGN KEY (start_vessel_event_id, start_vessel_event_type) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id) ON DELETE SET NULL;

ALTER TABLE trips
ADD CONSTRAINT end_vessel_event_fk FOREIGN KEY (end_vessel_event_id, end_vessel_event_type) REFERENCES vessel_events (vessel_event_id, vessel_event_type_id) ON DELETE SET NULL;

ALTER TABLE trips
ADD CONSTRAINT check_start_vessel_event_type_is_correct CHECK (
    (
        -- Landings assembler
        trip_assembler_id = 1
        -- Landing type event id
        AND start_vessel_event_type = 1
    )
    OR (
        -- Ers assembler
        trip_assembler_id = 2
        -- Departure type event id
        AND start_vessel_event_type = 4
    )
);

ALTER TABLE trips
ADD CONSTRAINT check_end_vessel_event_type_is_correct CHECK (
    (
        -- Landings assembler
        trip_assembler_id = 1
        -- Landing type event id
        AND end_vessel_event_type = 1
    )
    OR (
        -- Ers assembler
        trip_assembler_id = 2
        -- Por type event id
        AND end_vessel_event_type = 3
    )
);

-- Reset is required to populate the event id fields and to fix prior bugs in the trip assembler
UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;

CREATE INDEX ON trips (start_vessel_event_id);

CREATE INDEX ON trips (end_vessel_event_id);
