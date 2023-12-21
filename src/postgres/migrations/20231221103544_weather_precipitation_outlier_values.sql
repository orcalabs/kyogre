UPDATE weather
SET
    precipitation_amount = NULL
WHERE
    weather.precipitation_amount > 1000
    OR weather.precipitation_amount < -1000;

ALTER TABLE weather
ADD CONSTRAINT sane_precipitation_amount CHECK (
    precipitation_amount <= 1000
    AND precipitation_amount >= -1000
);
