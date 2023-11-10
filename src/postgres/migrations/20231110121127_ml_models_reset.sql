DELETE FROM ml_hauls_training_log;

UPDATE ml_models
SET
    model = NULL;
