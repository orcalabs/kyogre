UPDATE weather
SET
    air_pressure_at_sea_level = NULL
WHERE
    weather.air_pressure_at_sea_level > 300000
    OR weather.air_pressure_at_sea_level < 10000;

ALTER TABLE weather
ADD CONSTRAINT sane_air_pressure_at_sea_level CHECK (
    air_pressure_at_sea_level <= 300000
    AND air_pressure_at_sea_level >= 10000
);
