DELETE FROM engine_transitions;

DELETE FROM fishing_weight_predictions;

DELETE FROM fishing_spot_predictions;

DELETE FROM ml_hauls_training_log;

UPDATE ml_models
SET
    model = NULL;

ALTER TABLE fishing_weight_predictions
ADD COLUMN "date" date NOT NULL;

ALTER TABLE fishing_weight_predictions
DROP CONSTRAINT fishing_weight_predictions_pkey;

ALTER TABLE fishing_weight_predictions
ADD PRIMARY KEY (
    ml_model_id,
    catch_location_id,
    species_group_id,
    date
);

ALTER TABLE fishing_weight_predictions
DROP COLUMN week,
DROP COLUMN "year";

CREATE INDEX ON fishing_weight_predictions ("date");

DELETE FROM fishing_spot_predictions;

ALTER TABLE fishing_spot_predictions
ADD COLUMN "date" date NOT NULL;

ALTER TABLE fishing_spot_predictions
DROP CONSTRAINT fishing_spot_predictions_pkey;

ALTER TABLE fishing_spot_predictions
ADD PRIMARY KEY (ml_model_id, species_group_id, "date");

ALTER TABLE fishing_spot_predictions
DROP COLUMN week,
DROP COLUMN "year";

CREATE INDEX ON fishing_spot_predictions ("date");
