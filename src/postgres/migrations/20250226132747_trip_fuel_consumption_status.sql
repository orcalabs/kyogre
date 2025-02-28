ALTER TABLE trips
ADD COLUMN trip_position_fuel_consumption_distribution_status INT REFERENCES processing_status (processing_status_id);

UPDATE trips
SET
    track_coverage = COALESCE(track_coverage, 0),
    trip_position_fuel_consumption_distribution_status = 1;

ALTER TABLE trips
ALTER COLUMN track_coverage
SET NOT NULL,
ALTER COLUMN trip_position_fuel_consumption_distribution_status
SET NOT NULL;

UPDATE trips_detailed
SET
    track_coverage = 0
WHERE
    track_coverage IS NULL;

ALTER TABLE trips_detailed
ALTER COLUMN track_coverage
SET NOT NULL;

UPDATE trip_positions
SET
    trip_cumulative_cargo_weight = COALESCE(trip_cumulative_cargo_weight, 0),
    trip_cumulative_fuel_consumption_liter = COALESCE(trip_cumulative_fuel_consumption_liter, 0)
WHERE
    trip_cumulative_cargo_weight IS NULL
    OR trip_cumulative_fuel_consumption_liter IS NULL;

ALTER TABLE trip_positions
ALTER COLUMN trip_cumulative_cargo_weight
SET NOT NULL,
ALTER COLUMN trip_cumulative_fuel_consumption_liter
SET NOT NULL;
