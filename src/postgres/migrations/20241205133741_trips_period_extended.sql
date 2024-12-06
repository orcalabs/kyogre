ALTER TABLE trips
ADD COLUMN period_extended TSTZRANGE;

ALTER TABLE trips_detailed
ADD COLUMN period_extended TSTZRANGE;

UPDATE trips
SET
    period_extended = period;

UPDATE trips_detailed
SET
    period_extended = period;

ALTER TABLE trips
ALTER COLUMN period_extended
SET NOT NULL;

ALTER TABLE trips_detailed
ALTER COLUMN period_extended
SET NOT NULL;

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;
