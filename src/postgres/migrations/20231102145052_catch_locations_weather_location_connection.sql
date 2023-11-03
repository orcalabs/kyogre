ALTER TABLE catch_locations
ADD COLUMN weather_location_ids BIGINT[] NOT NULL DEFAULT '{}';

UPDATE catch_locations
SET
    weather_location_ids = q.weather_location_ids
FROM
    (
        SELECT
            catch_location_id,
            COALESCE(ARRAY_AGG(weather_location_id), '{}') AS weather_location_ids
        FROM
            catch_locations c
            INNER JOIN weather_locations w ON ST_OVERLAPS (c."polygon", w."polygon")
        GROUP BY
            c.catch_location_id
    ) q
WHERE
    q.catch_location_id = catch_locations.catch_location_id;

CREATE INDEX ON catch_locations USING gin (weather_location_ids);

ALTER TABLE fishing_weight_predictions
ADD COLUMN ml_model_id INT REFERENCES ml_models (ml_model_id);

UPDATE fishing_weight_predictions
SET
    ml_model_id = 2;

ALTER TABLE fishing_weight_predictions
ALTER COLUMN ml_model_id
SET NOT NULL;

INSERT INTO
    ml_models (ml_model_id, description)
VALUES
    (3, 'fishing_weight_weather_predictor');

ALTER TABLE fishing_weight_predictions
DROP CONSTRAINT fishing_weight_predictions_pkey;

CREATE UNIQUE INDEX fishing_weight_predictions_pkey ON fishing_weight_predictions (
    ml_model_id,
    catch_location_id,
    species_group_id,
    week,
    "year"
);

ALTER TABLE fishing_weight_predictions
ADD PRIMARY KEY USING INDEX fishing_weight_predictions_pkey;
