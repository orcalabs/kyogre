DROP TABLE fishing_weight_predictions;

DROP TABLE fishing_spot_predictions;

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'MLModels'
    OR destination = 'MLModels';

DELETE FROM engine_states
WHERE
    engine_state_id = 'MLModels';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('HaulWeather', 'VerifyDatabase');

DROP TABLE ml_hauls_training_log;

DROP TABLE ml_models;
