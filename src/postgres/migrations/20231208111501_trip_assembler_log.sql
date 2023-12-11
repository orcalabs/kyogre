CREATE TABLE
    trip_assembler_logs (
        trip_assembler_log_id bigserial PRIMARY KEY,
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        calculation_timer_prior timestamptz,
        calculation_timer_post timestamptz NOT NULL,
        "conflict" timestamptz,
        conflict_vessel_event_timestamp timestamptz,
        conflict_vessel_event_id BIGINT REFERENCES vessel_events (vessel_event_id) ON DELETE SET NULL,
        conflict_vessel_event_type_id INT REFERENCES vessel_event_types (vessel_event_type_id),
        prior_trip_vessel_events jsonb NOT NULL,
        new_vessel_events jsonb NOT NULL,
        CHECK (
            (
                "conflict" IS NULL
                AND conflict_vessel_event_type_id IS NULL
            )
            OR (
                "conflict" IS NOT NULL
                AND conflict_vessel_event_type_id IS NOT NULL
            )
        )
    );

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE,
    "conflict" = NULL;

ALTER TABLE trip_calculation_timers
ADD COLUMN conflict_vessel_event_id BIGINT REFERENCES vessel_events (vessel_event_id) ON DELETE SET NULL,
ADD COLUMN conflict_vessel_event_type_id INT REFERENCES vessel_event_types (vessel_event_type_id),
ADD COLUMN conflict_vessel_event_timestamp timestamptz,
ADD CONSTRAINT conflict_nullability CHECK (
    (
        "conflict" IS NULL
        AND conflict_vessel_event_type_id IS NULL
        AND conflict_vessel_event_timestamp IS NULL
    )
    OR (
        "conflict" IS NOT NULL
        AND conflict_vessel_event_type_id IS NOT NULL
        AND conflict_vessel_event_timestamp IS NOT NULL
    )
);
