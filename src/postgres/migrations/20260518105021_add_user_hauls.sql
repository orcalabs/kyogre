CREATE TABLE user_hauls (
    user_haul_id SERIAL PRIMARY KEY,
    fiskeridir_vessel_id INT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    barentswatch_user_id UUID NOT NULL,
    call_sign TEXT NOT NULL CHECK (call_sign != ''),
    start_ts TIMESTAMPTZ NOT NULL,
    end_ts TIMESTAMPTZ,
    start_fuel_liter INT NOT NULL CHECK (
        end_fuel_liter IS NULL
        OR start_fuel_liter > end_fuel_liter
    ),
    end_fuel_liter INT CHECK (
        (
            end_fuel_liter IS NULL
            AND end_ts IS NULL
        )
        OR (
            end_fuel_liter IS NOT NULL
            AND end_ts IS NOT NULL
        )
    ),
    period TSTZRANGE NOT NULL GENERATED ALWAYS AS (TSTZRANGE (start_ts, end_ts, '[]')) STORED,
    config JSONB NOT NULL
);

CREATE UNIQUE INDEX single_active_user_haul ON user_hauls (fiskeridir_vessel_id)
WHERE
    end_ts IS NULL;

ALTER TABLE user_hauls
ADD CONSTRAINT no_overlapping_user_hauls EXCLUDE USING gist (
    fiskeridir_vessel_id
    WITH
        =,
        period
    WITH
        &&
);
