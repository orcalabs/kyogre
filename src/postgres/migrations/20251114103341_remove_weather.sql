DROP TABLE weather;

DROP TABLE ocean_climate;

DROP TABLE weather_locations;

DROP TABLE weather_location_daily_weather;

DROP TABLE weather_location_types;

ALTER TABLE hauls
DROP COLUMN sea_floor_depth,
DROP COLUMN ocean_climate_depth,
DROP COLUMN water_temperature,
DROP COLUMN salinity,
DROP COLUMN water_direction,
DROP COLUMN water_speed,
DROP COLUMN haul_weather_status_id,
DROP COLUMN cloud_area_fraction,
DROP COLUMN precipitation_amount,
DROP COLUMN air_pressure_at_sea_level,
DROP COLUMN relative_humidity_2m,
DROP COLUMN air_temperature_2m,
DROP COLUMN wind_direction_10m,
DROP COLUMN wind_speed_10m;

DROP TABLE haul_weather_status;

DROP TABLE catch_location_daily_weather;

DROP TABLE daily_weather_dirty;

ALTER TABLE catch_locations
DROP COLUMN weather_location_ids;

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'DailyWeather'
    OR destination = 'DailyWeather'
    OR source = 'HaulWeather'
    OR destination = 'HaulWeather';

DELETE FROM engine_states
WHERE
    engine_state_id = 'DailyWeather'
    OR engine_state_id = 'HaulWeather';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Scrape', 'Trips'),
    ('HaulDistribution', 'VerifyDatabase');
