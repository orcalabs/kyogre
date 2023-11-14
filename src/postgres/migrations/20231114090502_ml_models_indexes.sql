CREATE INDEX ON fishing_weight_predictions (species_group_id);

CREATE INDEX ON fishing_weight_predictions (week);

CREATE INDEX ON fishing_weight_predictions (ml_model_id);

CREATE INDEX ON fishing_spot_predictions (week);

CREATE INDEX ON fishing_spot_predictions (species_group_id);

CREATE INDEX ON fishing_spot_predictions (ml_model_id);
