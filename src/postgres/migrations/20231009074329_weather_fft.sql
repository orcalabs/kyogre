CREATE TYPE fft_entry AS (idx INT, re DOUBLE PRECISION, im DOUBLE PRECISION);

CREATE TABLE
    weather_fft (
        "timestamp" TIMESTAMPTZ NOT NULL PRIMARY KEY,
        wind_speed_10m fft_entry[] NOT NULL,
        air_temperature_2m fft_entry[] NOT NULL,
        relative_humidity_2m fft_entry[] NOT NULL,
        air_pressure_at_sea_level fft_entry[] NOT NULL,
        precipitation_amount fft_entry[] NOT NULL
    );
