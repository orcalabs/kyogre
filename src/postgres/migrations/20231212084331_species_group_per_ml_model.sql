ALTER TABLE ml_models
ADD COLUMN species_group_id INT REFERENCES species_groups (species_group_id);

ALTER TABLE ml_models
DROP COLUMN description;

UPDATE ml_models
SET
    species_group_id = 201,
    model = NULL;

ALTER TABLE ml_models
ALTER COLUMN species_group_id
SET NOT NULL;

DELETE FROM ml_hauls_training_log;

DELETE FROM fishing_weight_predictions;

DELETE FROM fishing_spot_predictions;

ALTER TABLE ml_hauls_training_log
DROP CONSTRAINT ml_hauls_training_log_ml_model_id_fkey;

ALTER TABLE fishing_weight_predictions
DROP CONSTRAINT fishing_weight_predictions_ml_model_id_fkey;

ALTER TABLE fishing_spot_predictions
DROP CONSTRAINT fishing_spot_predictions_ml_model_id_fkey;

ALTER TABLE ml_models
DROP CONSTRAINT ml_models_pkey;

ALTER TABLE ml_models
ADD PRIMARY KEY (ml_model_id, species_group_id);

INSERT INTO
    ml_models (ml_model_id, species_group_id)
VALUES
    (1, 202),
    (1, 203),
    (1, 505),
    (2, 202),
    (2, 203),
    (2, 505),
    (3, 202),
    (3, 203),
    (3, 505),
    (4, 202),
    (4, 203),
    (4, 505);

ALTER TABLE ml_hauls_training_log
ADD CONSTRAINT ml_hauls_training_log_ml_model_id_fkey FOREIGN KEY (ml_model_id, species_group_id) REFERENCES ml_models (ml_model_id, species_group_id);

ALTER TABLE fishing_weight_predictions
ADD CONSTRAINT fishing_weight_predictions_ml_model_id_fkey FOREIGN KEY (ml_model_id, species_group_id) REFERENCES ml_models (ml_model_id, species_group_id);

ALTER TABLE fishing_spot_predictions
ADD CONSTRAINT fishing_spot_predictions_ml_model_id_fkey FOREIGN KEY (ml_model_id, species_group_id) REFERENCES ml_models (ml_model_id, species_group_id);
