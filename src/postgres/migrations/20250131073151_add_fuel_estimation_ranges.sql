CREATE
OR REPLACE FUNCTION truncate_ts_to_day (ts TIMESTAMPTZ) RETURNS TIMESTAMPTZ AS $$
BEGIN
    RETURN DATE_TRUNC('day', ts AT TIME ZONE 'UTC');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION generate_pre_estimate_ts (start_measurement_ts TIMESTAMPTZ) RETURNS TIMESTAMPTZ AS $$
DECLARE
    _ts TIMESTAMPTZ;
BEGIN
    _ts = truncate_ts_to_day(start_measurement_ts);
    RETURN CASE
        WHEN _ts = start_measurement_ts THEN start_measurement_ts
        ELSE _ts + INTERVAL '1 day - 1 microsecond'
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION cast_date_to_ts (input DATE) RETURNS TIMESTAMPTZ AS $$
BEGIN
    RETURN input::TIMESTAMPTZ;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

DROP TABLE fuel_measurements;

CREATE TABLE fuel_measurements (
    fuel_measurement_id BIGSERIAL PRIMARY KEY,
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    barentswatch_user_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    fuel DOUBLE PRECISION NOT NULL,
    UNIQUE (fiskeridir_vessel_id, timestamp)
);

CREATE TABLE fuel_measurement_ranges (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    fuel_range TSTZRANGE NOT NULL CHECK (fuel_range != 'empty') GENERATED ALWAYS AS (
        TSTZRANGE (start_measurement_ts, end_measurement_ts, '()')
    ) STORED,
    fuel_used DOUBLE PRECISION NOT NULL CHECK (fuel_used >= 0.0) GENERATED ALWAYS AS (
        GREATEST(
            start_measurement_fuel - end_measurement_fuel,
            0.0
        )
    ) STORED,
    start_measurement_fuel DOUBLE PRECISION NOT NULL,
    end_measurement_fuel DOUBLE PRECISION NOT NULL,
    start_measurement_ts TIMESTAMPTZ NOT NULL,
    end_measurement_ts TIMESTAMPTZ NOT NULL,
    pre_estimate_ts TIMESTAMPTZ NOT NULL GENERATED ALWAYS AS (generate_pre_estimate_ts (start_measurement_ts)) STORED,
    post_estimate_ts TIMESTAMPTZ NOT NULL GENERATED ALWAYS AS (truncate_ts_to_day (end_measurement_ts)) STORED,
    pre_estimate_value DOUBLE PRECISION CHECK (
        (
            pre_estimate_value IS NOT NULL
            AND pre_estimate_ts != start_measurement_ts
        )
        OR TRUE
    ),
    post_estimate_value DOUBLE PRECISION CHECK (
        (
            post_estimate_value IS NOT NULL
            AND post_estimate_ts != end_measurement_ts
        )
        OR TRUE
    ),
    EXCLUDE USING gist (
        fiskeridir_vessel_id
        WITH
            =,
            fuel_range
        WITH
            &&
    ),
    pre_post_estimate_status INT NOT NULL REFERENCES processing_status (processing_status_id),
    PRIMARY KEY (fiskeridir_vessel_id, fuel_range),
    FOREIGN KEY (fiskeridir_vessel_id, start_measurement_ts) REFERENCES fuel_measurements (fiskeridir_vessel_id, timestamp) ON DELETE CASCADE,
    FOREIGN KEY (fiskeridir_vessel_id, end_measurement_ts) REFERENCES fuel_measurements (fiskeridir_vessel_id, timestamp) ON DELETE CASCADE
);

CREATE INDEX ON fuel_estimates (fiskeridir_vessel_id, CAST_DATE_TO_TS (date));

CREATE INDEX ON fuel_measurement_ranges (fiskeridir_vessel_id, start_measurement_ts);

CREATE INDEX ON fuel_measurement_ranges (fiskeridir_vessel_id, end_measurement_ts);
