UPDATE weather
SET
    wind_speed_10m = NULL
WHERE
    weather.wind_speed_10m > 1000
    OR weather.wind_speed_10m < 0;

UPDATE weather
SET
    air_temperature_2m = NULL
WHERE
    weather.air_temperature_2m > 1000
    OR weather.air_temperature_2m < -1000;

UPDATE weather
SET
    precipitation_amount = NULL
WHERE
    weather.precipitation_amount > 1000
    OR weather.precipitation_amount < -1000;

UPDATE weather
SET
    air_pressure_at_sea_level = NULL
WHERE
    weather.air_pressure_at_sea_level > 300000
    OR weather.air_pressure_at_sea_level < 10000;

UPDATE weather
SET
    relative_humidity_2m = 1
WHERE
    weather.relative_humidity_2m > 1;

UPDATE weather
SET
    relative_humidity_2m = NULL
WHERE
    weather.relative_humidity_2m < 0;

UPDATE weather
SET
    cloud_area_fraction = 1
WHERE
    weather.cloud_area_fraction > 1;

UPDATE weather
SET
    cloud_area_fraction = NULL
WHERE
    weather.cloud_area_fraction < 0;

ALTER TABLE weather
ADD CONSTRAINT sane_wind_speed_10m CHECK (
    wind_speed_10m <= 1000
    AND wind_speed_10m >= 0
);

ALTER TABLE weather
ADD CONSTRAINT sane_air_temperature_2m CHECK (
    air_temperature_2m <= 1000
    AND air_temperature_2m >= -1000
);

ALTER TABLE weather
ADD CONSTRAINT sane_precipitation_amount CHECK (
    precipitation_amount <= 1000
    AND precipitation_amount >= -1000
);

ALTER TABLE weather
ADD CONSTRAINT sane_air_pressure_at_sea_level CHECK (
    air_pressure_at_sea_level <= 300000
    AND air_pressure_at_sea_level >= 10000
);

ALTER TABLE weather
ADD CONSTRAINT sane_relative_humidity_2m CHECK (
    relative_humidity_2m <= 1
    AND relative_humidity_2m >= 0
);

ALTER TABLE weather
ADD CONSTRAINT sane_cloud_area_fraction CHECK (
    cloud_area_fraction <= 1
    AND cloud_area_fraction >= 0
);

ALTER TABLE weather
ADD CONSTRAINT sane_land_area_fraction CHECK (
    land_area_fraction <= 1
    AND land_area_fraction >= 0
);
