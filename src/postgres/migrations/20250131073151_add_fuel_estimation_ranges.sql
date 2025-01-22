CREATE
OR REPLACE FUNCTION compute_fuel_used (
    start_measurement_fuel DOUBLE PRECISION,
    start_measurement_fuel_after DOUBLE PRECISION,
    end_measurement_fuel DOUBLE PRECISION
) RETURNS DOUBLE PRECISION AS $$
BEGIN
    RETURN COALESCE(
        start_measurement_fuel_after,
        start_measurement_fuel
    ) - end_measurement_fuel;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION truncate_ts_to_day (ts TIMESTAMPTZ) RETURNS TIMESTAMPTZ AS $$
BEGIN
    RETURN DATE_TRUNC('day', ts AT TIME ZONE 'UTC');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION generate_day_range_for_ts (ts TIMESTAMPTZ) RETURNS TSTZRANGE AS $$
BEGIN
    RETURN TSTZRANGE (ts, ts + INTERVAL '1 day', '[)');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION len_of_range (a TSTZRANGE) RETURNS INTERVAL AS $$
BEGIN
    RETURN UPPER(a) - LOWER(a);
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION len_of_multirange (a TSTZMULTIRANGE) RETURNS INTERVAL AS $$
   SELECT SUM(len_of_range(r)) FROM UNNEST(a) as r;
$$ LANGUAGE sql IMMUTABLE;

CREATE
OR REPLACE FUNCTION compute_ts_range_percent_overlap (a TSTZRANGE, b TSTZRANGE) RETURNS DOUBLE PRECISION AS $$
BEGIN
    RETURN CASE WHEN a @> b THEN 1.0
    ELSE COALESCE(EXTRACT('epoch' FROM len_of_range(a * b)) / NULLIF(EXTRACT('epoch' FROM len_of_range(a)), 0), 0.0)
    END;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE
OR REPLACE FUNCTION compute_ts_range_mutlirange_percent_overlap (a TSTZRANGE, b TSTZMULTIRANGE) RETURNS DOUBLE PRECISION AS $$
DECLARE
    _intersect TSTZMULTIRANGE;
BEGIN
    _intersect = a::TSTZMULTIRANGE * b;
    RETURN COALESCE(EXTRACT('epoch' FROM len_of_multirange(_intersect)) / NULLIF(EXTRACT('epoch' FROM len_of_range(a)), 0), 0.0);
END;
$$ LANGUAGE plpgsql IMMUTABLE;

ALTER TABLE fuel_estimates
ALTER COLUMN "date"
SET DATA TYPE TIMESTAMPTZ USING "date"::TIMESTAMPTZ,
ADD CONSTRAINT date_is_start_of_day CHECK ("date" = truncate_ts_to_day ("date"));

ALTER TABLE fuel_estimates
ADD COLUMN day_range TSTZRANGE NOT NULL GENERATED ALWAYS AS (generate_day_range_for_ts ("date")) STORED;

DROP TABLE fuel_measurements;

CREATE TABLE fuel_measurements (
    fuel_measurement_id BIGSERIAL PRIMARY KEY,
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    barentswatch_user_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    fuel DOUBLE PRECISION NOT NULL,
    fuel_after DOUBLE PRECISION CHECK (
        fuel_after IS NULL
        OR fuel_after > fuel
    ),
    UNIQUE (fiskeridir_vessel_id, timestamp)
);

CREATE TABLE fuel_measurement_ranges (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    fuel_range TSTZRANGE NOT NULL CHECK (NOT isempty(fuel_range)) GENERATED ALWAYS AS (
        TSTZRANGE (start_measurement_ts, end_measurement_ts, '[]')
    ) STORED,
    fuel_range_excluded TSTZRANGE NOT NULL CHECK (NOT isempty(fuel_range_excluded)) GENERATED ALWAYS AS (
        TSTZRANGE (start_measurement_ts, end_measurement_ts, '()')
    ) STORED,
    fuel_used DOUBLE PRECISION NOT NULL CHECK (fuel_used > 0.0) GENERATED ALWAYS AS (
        compute_fuel_used (
            start_measurement_fuel,
            start_measurement_fuel_after,
            end_measurement_fuel
        )
    ) STORED,
    start_measurement_fuel_after DOUBLE PRECISION CHECK (
        start_measurement_fuel_after IS NULL
        OR start_measurement_fuel_after > start_measurement_fuel
    ),
    start_measurement_fuel DOUBLE PRECISION NOT NULL,
    end_measurement_fuel DOUBLE PRECISION NOT NULL,
    start_measurement_ts TIMESTAMPTZ NOT NULL,
    end_measurement_ts TIMESTAMPTZ NOT NULL,
    EXCLUDE USING gist (
        fiskeridir_vessel_id
        WITH
            =,
            fuel_range_excluded
        WITH
            &&
    ),
    PRIMARY KEY (fiskeridir_vessel_id, fuel_range),
    FOREIGN KEY (fiskeridir_vessel_id, start_measurement_ts) REFERENCES fuel_measurements (fiskeridir_vessel_id, timestamp) ON DELETE CASCADE,
    FOREIGN KEY (fiskeridir_vessel_id, end_measurement_ts) REFERENCES fuel_measurements (fiskeridir_vessel_id, timestamp) ON DELETE CASCADE
);

CREATE INDEX ON fuel_measurement_ranges (fiskeridir_vessel_id, start_measurement_ts);

CREATE INDEX ON fuel_measurement_ranges (fiskeridir_vessel_id, end_measurement_ts);

CREATE INDEX ON fuel_estimates USING GIST (fiskeridir_vessel_id, day_range);
