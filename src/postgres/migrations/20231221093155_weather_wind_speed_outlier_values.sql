UPDATE weather
SET
    wind_speed_10m = NULL
WHERE
    weather.wind_speed_10m > 1000
    OR weather.wind_speed_10m < 0;

ALTER TABLE weather
ADD CONSTRAINT sane_wind_speed_10m CHECK (
    wind_speed_10m <= 1000
    AND wind_speed_10m >= 0
);
