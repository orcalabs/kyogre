ALTER TABLE trip_calculation_timers
ADD COLUMN queued_reset BOOLEAN DEFAULT FALSE;
