CREATE TABLE
    weather_images (
        "timestamp" TIMESTAMPTZ NOT NULL PRIMARY KEY,
        wind_speed_10m BYTEA NOT NULL,
        air_temperature_2m BYTEA NOT NULL,
        relative_humidity_2m BYTEA NOT NULL,
        air_pressure_at_sea_level BYTEA NOT NULL,
        precipitation_amount BYTEA NOT NULL
    );
