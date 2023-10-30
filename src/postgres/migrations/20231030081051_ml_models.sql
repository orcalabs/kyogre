INSERT INTO
    engine_states
VALUES
    ('MLModels');

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'HaulWeather'
    AND destination = 'VerifyDatabase';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('HaulWeather', 'MLModels'),
    ('MLModels', 'VerifyDatabase'),
    ('MLModels', 'Pending'),
    ('Pending', 'MLModels');

CREATE TABLE
    ml_models (
        ml_model_id INT PRIMARY KEY,
        description VARCHAR NOT NULL,
        model bytea
    );

INSERT INTO
    ml_models (ml_model_id, description)
VALUES
    (1, 'fishing_spot_predictor'),
    (2, 'fishing_weight_predictor');

CREATE TABLE
    ml_hauls_training_log (
        ml_model_id INT REFERENCES ml_models (ml_model_id) NOT NULL,
        haul_id INT REFERENCES hauls (haul_id) ON DELETE CASCADE NOT NULL,
        PRIMARY KEY (ml_model_id, haul_id)
    );

CREATE INDEX ON ml_hauls_training_log (haul_id);

CREATE TABLE
    fishing_spot_predictions (
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        week INT NOT NULL,
        "year" INT NOT NULL,
        latitude DOUBLE PRECISION NOT NULL,
        longitude DOUBLE PRECISION NOT NULL,
        PRIMARY KEY (species_group_id, week, "year")
    );

CREATE TABLE
    fishing_weight_predictions (
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id),
        week INT NOT NULL,
        "year" INT NOT NULL,
        catch_location_id VARCHAR NOT NULL REFERENCES catch_locations (catch_location_id),
        weight DOUBLE PRECISION NOT NULL,
        PRIMARY KEY (catch_location_id, species_group_id, week, "year")
    );

ALTER TABLE catch_locations
ADD COLUMN latitude DOUBLE PRECISION GENERATED ALWAYS AS (ST_Y (ST_CENTROID ("polygon"))) STORED NOT NULL,
ADD COLUMN longitude DOUBLE PRECISION GENERATED ALWAYS AS (ST_X (ST_CENTROID ("polygon"))) STORED NOT NULL;
