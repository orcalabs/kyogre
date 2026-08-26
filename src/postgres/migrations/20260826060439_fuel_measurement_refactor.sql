DELETE FROM fuel_measurement_ranges;

--! All fuel refills represent an additional 'fuel_measurements' row in our new scheme.
--! The 1 second addition to the timestamp is an arbitrary value and is needed to not collide on unique indexes.
INSERT INTO
    fuel_measurements (
        fiskeridir_vessel_id,
        barentswatch_user_id,
        timestamp,
        fuel_liter
    )
SELECT
    fiskeridir_vessel_id,
    barentswatch_user_id,
    timestamp + INTERVAL '1 second',
    fuel_after_liter
FROM
    fuel_measurements
WHERE
    fuel_after_liter IS NOT NULL;

ALTER TABLE fuel_measurements
DROP COLUMN fuel_after_liter;

ALTER TABLE fuel_measurement_ranges
DROP COLUMN fuel_used_liter,
DROP COLUMN start_measurement_fuel_after_liter;

ALTER TABLE fuel_measurement_ranges
ADD COLUMN fuel_used_liter DOUBLE PRECISION NOT NULL GENERATED ALWAYS AS (
    end_measurement_fuel_liter - start_measurement_fuel_liter
) STORED,
ADD COLUMN is_reset BOOLEAN NOT NULL GENERATED ALWAYS AS (
    end_measurement_fuel_liter - start_measurement_fuel_liter <= 0.0
) STORED;

--! Reverse all measurements for our new incrementing fuel scheme.
--! To ensure that the first entry that the user inputs does not cause a big spike in fuel usage
--! we use a big number here which will cause a reset as their fuel counter is *PROBABLY* lower.
--! If we did not use a big number the first entry of a user might be higher than our artifical number and
--! be registered as a normal increase in fuel.
UPDATE fuel_measurements
SET
    fuel_liter = 2147483647 - fuel_liter;

ALTER TABLE user_hauls
DROP CONSTRAINT user_hauls_check;

UPDATE user_hauls
SET
    start_fuel_liter = end_fuel_liter,
    end_fuel_liter = start_fuel_liter
WHERE
    end_fuel_liter IS NOT NULL;

WITH
    measurements AS (
        SELECT
            fiskeridir_vessel_id,
            fuel_liter,
            timestamp,
            LEAD(fuel_liter) OVER (
                PARTITION BY
                    fiskeridir_vessel_id
                ORDER BY
                    timestamp
            ) AS next_liter,
            LEAD(timestamp) OVER (
                PARTITION BY
                    fiskeridir_vessel_id
                ORDER BY
                    timestamp
            ) AS next_ts
        FROM
            fuel_measurements
    )
INSERT INTO
    fuel_measurement_ranges (
        fiskeridir_vessel_id,
        start_measurement_fuel_liter,
        start_measurement_ts,
        end_measurement_fuel_liter,
        end_measurement_ts
    )
SELECT
    *
FROM
    measurements
WHERE
    next_ts IS NOT NULL;

WITH
    vessels AS (
        SELECT
            fiskeridir_vessel_id,
            MIN(start_ts) AS start_ts
        FROM
            user_hauls
        GROUP BY
            fiskeridir_vessel_id
    )
UPDATE trips_refresh_boundary t
SET
    refresh_boundary = v.start_ts
FROM
    vessels v
WHERE
    v.fiskeridir_vessel_id = t.fiskeridir_vessel_id;
