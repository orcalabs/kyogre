UPDATE fuel_estimates
SET
    estimate = estimate * 1000 * 1.163;

UPDATE live_fuel
SET
    fuel = fuel * 1000 * 1.163;

UPDATE trip_positions
SET
    trip_cumulative_fuel_consumption = trip_cumulative_fuel_consumption * 1000 * 1.163
WHERE
    trip_cumulative_fuel_consumption IS NOT NULL;

UPDATE trips_detailed
SET
    benchmark_fuel_consumption = benchmark_fuel_consumption * 1000 * 1.163,
    benchmark_weight_per_fuel = benchmark_weight_per_fuel / 1000 / 1.163,
    benchmark_catch_value_per_fuel = benchmark_catch_value_per_fuel / 1000 / 1.163
WHERE
    benchmark_fuel_consumption IS NOT NULL
    OR benchmark_weight_per_fuel IS NOT NULL
    OR benchmark_catch_value_per_fuel IS NOT NULL;

ALTER TABLE fuel_estimates
RENAME COLUMN estimate TO estimate_liter;

ALTER TABLE live_fuel
RENAME COLUMN fuel TO fuel_liter;

ALTER TABLE trip_positions
RENAME COLUMN trip_cumulative_fuel_consumption TO trip_cumulative_fuel_consumption_liter;

ALTER TABLE trips_detailed
RENAME COLUMN benchmark_fuel_consumption TO benchmark_fuel_consumption_liter;

ALTER TABLE trips_detailed
RENAME COLUMN benchmark_weight_per_fuel TO benchmark_weight_per_fuel_liter;

ALTER TABLE trips_detailed
RENAME COLUMN benchmark_catch_value_per_fuel TO benchmark_catch_value_per_fuel_liter;

ALTER TABLE fuel_measurements
RENAME COLUMN fuel TO fuel_liter;

ALTER TABLE fuel_measurements
RENAME COLUMN fuel_after TO fuel_after_liter;

ALTER TABLE fuel_measurement_ranges
RENAME COLUMN fuel_used TO fuel_used_liter;

ALTER TABLE fuel_measurement_ranges
RENAME COLUMN start_measurement_fuel_after TO start_measurement_fuel_after_liter;

ALTER TABLE fuel_measurement_ranges
RENAME COLUMN start_measurement_fuel TO start_measurement_fuel_liter;

ALTER TABLE fuel_measurement_ranges
RENAME COLUMN end_measurement_fuel TO end_measurement_fuel_liter;
