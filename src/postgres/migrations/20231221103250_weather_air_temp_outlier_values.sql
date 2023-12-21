UPDATE weather
SET
    air_temperature_2m = NULL
WHERE
    weather.air_temperature_2m > 1000
    OR weather.air_temperature_2m < -1000;

ALTER TABLE weather
ADD CONSTRAINT sane_air_temperature_2m CHECK (
    air_temperature_2m <= 1000
    AND air_temperature_2m >= -1000
);
