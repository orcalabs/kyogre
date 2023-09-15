TRUNCATE TABLE engine_transitions;

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('HaulWeather');

DELETE FROM valid_engine_transitions
WHERE
    source = 'HaulDistribution'
    AND destination = 'TripDistance';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Pending', 'HaulWeather'),
    ('HaulDistribution', 'HaulWeather'),
    ('HaulWeather', 'TripDistance');

CREATE TABLE
    haul_weather_status (
        haul_weather_status_id INT PRIMARY KEY,
        description TEXT NOT NULL
    );

INSERT INTO
    haul_weather_status (haul_weather_status_id, description)
VALUES
    (1, 'unprocessed'),
    (2, 'attempted'),
    (3, 'successful');

ALTER TABLE hauls
ADD COLUMN wind_speed_10m DECIMAL,
ADD COLUMN wind_direction_10m DECIMAL,
ADD COLUMN air_temperature_2m DECIMAL,
ADD COLUMN relative_humidity_2m DECIMAL,
ADD COLUMN air_pressure_at_sea_level DECIMAL,
ADD COLUMN precipitation_amount DECIMAL,
ADD COLUMN cloud_area_fraction DECIMAL,
ADD COLUMN haul_weather_status_id INT NOT NULL DEFAULT (1) REFERENCES haul_weather_status (haul_weather_status_id);
