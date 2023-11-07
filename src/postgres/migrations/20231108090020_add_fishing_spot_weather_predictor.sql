ALTER TABLE fishing_spot_predictions
ADD COLUMN ml_model_id INT REFERENCES ml_models (ml_model_id);

UPDATE fishing_spot_predictions
SET
    ml_model_id = 1;

ALTER TABLE fishing_spot_predictions
ALTER COLUMN ml_model_id
SET NOT NULL;

INSERT INTO
    ml_models (ml_model_id, description)
VALUES
    (4, 'fishing_spot_weather_predictor');

ALTER TABLE fishing_spot_predictions
DROP CONSTRAINT fishing_spot_predictions_pkey;

CREATE UNIQUE INDEX fishing_spot_predictions_pkey ON fishing_spot_predictions (ml_model_id, species_group_id, week, "year");

ALTER TABLE fishing_spot_predictions
ADD PRIMARY KEY USING INDEX fishing_spot_predictions_pkey;

CREATE INDEX ON weather (weather_location_id);

UPDATE ml_models
SET
    model = NULL;

DELETE FROM ml_hauls_training_log;
