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
ADD CONSTRAINT sane_cloud_area_fraction CHECK (
    cloud_area_fraction <= 1
    AND cloud_area_fraction >= 0
);
