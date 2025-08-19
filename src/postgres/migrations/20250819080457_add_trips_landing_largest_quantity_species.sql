ALTER TABLE trips_detailed
ADD COLUMN landing_largest_quantum_species_group_id INT REFERENCES species_groups (species_group_id);

UPDATE trip_calculation_timers
SET
    queued_reset = TRUE;
