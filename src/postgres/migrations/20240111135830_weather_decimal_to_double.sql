ALTER TABLE weather
DROP CONSTRAINT weather_pkey,
DROP COLUMN weather_location_id,
ALTER COLUMN latitude TYPE DOUBLE PRECISION,
ALTER COLUMN longitude TYPE DOUBLE PRECISION,
ALTER COLUMN altitude TYPE DOUBLE PRECISION,
ALTER COLUMN wind_speed_10m TYPE DOUBLE PRECISION,
ALTER COLUMN wind_direction_10m TYPE DOUBLE PRECISION,
ALTER COLUMN air_temperature_2m TYPE DOUBLE PRECISION,
ALTER COLUMN relative_humidity_2m TYPE DOUBLE PRECISION,
ALTER COLUMN air_pressure_at_sea_level TYPE DOUBLE PRECISION,
ALTER COLUMN precipitation_amount TYPE DOUBLE PRECISION,
ALTER COLUMN land_area_fraction TYPE DOUBLE PRECISION,
ALTER COLUMN cloud_area_fraction TYPE DOUBLE PRECISION,
ADD COLUMN weather_location_id INT NOT NULL REFERENCES weather_locations (weather_location_id) GENERATED ALWAYS AS (
    ((FLOOR(latitude / 0.1) + 1000) * 100000) + (FLOOR(longitude / 0.1) + 1000)
) STORED,
ADD PRIMARY KEY ("timestamp", weather_location_id);

ALTER TABLE ocean_climate
DROP CONSTRAINT ocean_climate_pkey,
DROP COLUMN weather_location_id,
ALTER COLUMN latitude TYPE DOUBLE PRECISION,
ALTER COLUMN longitude TYPE DOUBLE PRECISION,
ALTER COLUMN water_speed TYPE DOUBLE PRECISION,
ALTER COLUMN water_direction TYPE DOUBLE PRECISION,
ALTER COLUMN upward_sea_velocity TYPE DOUBLE PRECISION,
ALTER COLUMN wind_speed TYPE DOUBLE PRECISION,
ALTER COLUMN wind_direction TYPE DOUBLE PRECISION,
ALTER COLUMN salinity TYPE DOUBLE PRECISION,
ALTER COLUMN temperature TYPE DOUBLE PRECISION,
ALTER COLUMN sea_floor_depth TYPE DOUBLE PRECISION,
ADD COLUMN weather_location_id INT NOT NULL REFERENCES weather_locations (weather_location_id) GENERATED ALWAYS AS (
    ((FLOOR(latitude / 0.1) + 1000) * 100000) + (FLOOR(longitude / 0.1) + 1000)
) STORED,
ADD PRIMARY KEY ("timestamp", "depth", weather_location_id);

ALTER TABLE vessel_benchmark_outputs
ALTER COLUMN output TYPE DOUBLE PRECISION;
