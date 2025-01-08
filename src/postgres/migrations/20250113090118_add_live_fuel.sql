CREATE TABLE live_fuel (
    fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE cascade,
    latest_position_timestamp TIMESTAMPTZ NOT NULL CHECK (
        DATE_PART(
            'year',
            latest_position_timestamp AT TIME ZONE 'UTC'
        ) = "year"
        AND DATE_PART(
            'doy',
            latest_position_timestamp AT TIME ZONE 'UTC'
        ) = "day"
        AND DATE_PART(
            'hour',
            latest_position_timestamp AT TIME ZONE 'UTC'
        ) = "hour"
    ),
    fuel DOUBLE PRECISION NOT NULL,
    "year" int,
    "day" int,
    "hour" int,
    PRIMARY KEY (fiskeridir_vessel_id, "year", "day", "hour")
);

CREATE INDEX ON live_fuel (fiskeridir_vessel_id, latest_position_timestamp);
