DROP TABLE ais_area;

CREATE
OR REPLACE FUNCTION TIMESTAMP_RANGE (ts timestamptz, size INTERVAL) RETURNS tstzrange LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN (tstzrange ((ts - size), (ts + size), '()'));
    END;
$$;

CREATE TABLE
    ais_vms_area_aggregated (
        latitude DECIMAL(10, 2) NOT NULL,
        longitude DECIMAL(10, 2) NOT NULL,
        date DATE NOT NULL,
        "count" INT NOT NULL,
        mmsis INT[] NOT NULL,
        call_signs VARCHAR[] NOT NULL,
        PRIMARY KEY (latitude, longitude, date)
    );

CREATE TABLE
    ais_vms_area_positions (
        latitude DOUBLE PRECISION NOT NULL,
        longitude DOUBLE PRECISION NOT NULL,
        call_sign VARCHAR,
        mmsi INT,
        "timestamp" timestamptz NOT NULL,
        position_type_id INT NOT NULL REFERENCES position_types (position_type_id),
        duplicate_range tstzrange NOT NULL GENERATED ALWAYS AS (
            TIMESTAMP_RANGE ("timestamp", INTERVAL '20 seconds')
        ) STORED,
        CONSTRAINT duplicate_positions EXCLUDE USING gist (
            call_sign
            WITH
                =,
                duplicate_range
            WITH
                &&
        )
        WHERE
            (call_sign IS NOT NULL)
    );

CREATE INDEX ON ais_vms_area_positions ("timestamp");

CREATE INDEX ON ais_vms_area_aggregated USING GIST (ST_POINT (longitude, latitude));

CREATE INDEX ON ais_vms_area_aggregated (date);

CREATE INDEX ON ais_vms_area_aggregated (latitude, longitude);
