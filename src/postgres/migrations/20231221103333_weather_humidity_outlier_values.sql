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

ALTER TABLE weather
ADD CONSTRAINT sane_relative_humidity_2m CHECK (
    relative_humidity_2m <= 1
    AND relative_humidity_2m >= 0
);
