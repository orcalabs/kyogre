DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'Scrape'
    AND destination = 'Trips';

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('CatchLocationWeather');

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Scrape', 'CatchLocationWeather'),
    ('Pending', 'CatchLocationWeather'),
    ('CatchLocationWeather', 'Trips');

CREATE TABLE
    catch_location_daily_weather_dirty (date date PRIMARY KEY);

CREATE
OR REPLACE FUNCTION CL_WEATHER_DAY (catch_location_id VARCHAR, weather_date date) RETURNS VARCHAR LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN (
            SELECT catch_location_id || '-' || weather_date
        );
    END;
$$;

CREATE TABLE
    catch_location_daily_weather (
        catch_location_id VARCHAR NOT NULL REFERENCES catch_locations (catch_location_id),
        date date NOT NULL,
        catch_location_daily_weather_id VARCHAR GENERATED ALWAYS AS (CL_WEATHER_DAY (catch_location_id, date)) STORED PRIMARY KEY,
        altitude DOUBLE PRECISION NOT NULL,
        wind_speed_10m DOUBLE PRECISION NOT NULL,
        wind_direction_10m DOUBLE PRECISION NOT NULL,
        air_temperature_2m DOUBLE PRECISION NOT NULL,
        relative_humidity_2m DOUBLE PRECISION NOT NULL,
        air_pressure_at_sea_level DOUBLE PRECISION NOT NULL,
        precipitation_amount DOUBLE PRECISION NOT NULL,
        cloud_area_fraction DOUBLE PRECISION NOT NULL,
        UNIQUE (catch_location_id, date)
    );

CREATE INDEX ON catch_location_daily_weather (date);

INSERT INTO
    catch_location_daily_weather_dirty
SELECT
    DATE_TRUNC('day', dd)::date
FROM
    GENERATE_SERIES(
        '2013-01-01'::TIMESTAMP,
        '2023-12-20'::TIMESTAMP,
        '1 day'::INTERVAL
    ) dd;
