ALTER TABLE trips_detailed
DROP COLUMN haul_total_weight,
DROP COLUMN haul_duration;

ALTER TABLE trips_detailed
ADD COLUMN haul_total_weight int,
ADD COLUMN haul_duration interval;

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;

DROP FUNCTION sum_haul_duration;
