ALTER TABLE weather
ADD CONSTRAINT sane_land_area_fraction CHECK (
    land_area_fraction <= 1
    AND land_area_fraction >= 0
);
