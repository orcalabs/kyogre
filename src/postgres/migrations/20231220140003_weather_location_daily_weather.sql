CREATE
OR REPLACE FUNCTION WL_WEATHER_DAY (weather_location_id INT, weather_date date) RETURNS VARCHAR LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN (
            SELECT weather_location_id || '-' || weather_date
        );
    END;
$$;

CREATE TABLE
    weather_location_daily_weather (
        weather_location_id INT NOT NULL,
        date date NOT NULL,
        weather_location_daily_weather_id VARCHAR GENERATED ALWAYS AS (WL_WEATHER_DAY (weather_location_id, date)) STORED PRIMARY KEY,
        altitude DOUBLE PRECISION NOT NULL,
        wind_speed_10m DOUBLE PRECISION NOT NULL,
        wind_direction_10m DOUBLE PRECISION NOT NULL,
        air_temperature_2m DOUBLE PRECISION NOT NULL,
        relative_humidity_2m DOUBLE PRECISION NOT NULL,
        air_pressure_at_sea_level DOUBLE PRECISION NOT NULL,
        precipitation_amount DOUBLE PRECISION NOT NULL,
        cloud_area_fraction DOUBLE PRECISION NOT NULL,
        UNIQUE (weather_location_id, date)
    );

ALTER TABLE catch_location_daily_weather_dirty
RENAME TO daily_weather_dirty;

INSERT INTO
    daily_weather_dirty
SELECT
    DATE_TRUNC('day', dd)::date
FROM
    GENERATE_SERIES(
        '2013-01-01'::TIMESTAMP,
        '2023-12-20'::TIMESTAMP,
        '1 day'::INTERVAL
    ) dd
ON CONFLICT (date) DO NOTHING;

DELETE FROM engine_transitions;

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('DailyWeather');

UPDATE valid_engine_transitions
SET
    source = 'DailyWeather'
WHERE
    source = 'CatchLocationWeather';

UPDATE valid_engine_transitions
SET
    destination = 'DailyWeather'
WHERE
    destination = 'CatchLocationWeather';

DELETE FROM engine_states
WHERE
    engine_state_id = 'CatchLocationWeather';
