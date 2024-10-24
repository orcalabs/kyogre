ALTER TABLE trips
ADD COLUMN track_coverage DOUBLE PRECISION CHECK (
    track_coverage IS NULL
    OR track_coverage BETWEEN 0 AND 100
);

ALTER TABLE trips_detailed
ADD COLUMN track_coverage DOUBLE PRECISION CHECK (
    track_coverage IS NULL
    OR track_coverage BETWEEN 0 AND 100
);

UPDATE trips
SET
    position_layers_status = 1;
