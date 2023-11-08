UPDATE ml_models
SET
    model = NULL;

DROP TABLE ml_hauls_training_log;

CREATE TABLE
    ml_hauls_training_log (
        ml_model_id INT NOT NULL REFERENCES ml_models (ml_model_id) ON DELETE CASCADE,
        haul_id BIGINT NOT NULL REFERENCES hauls (haul_id) ON DELETE CASCADE,
        species_group_id INT NOT NULL REFERENCES species_groups (species_group_id) ON DELETE CASCADE,
        catch_location_id VARCHAR NOT NULL REFERENCES catch_locations (catch_location_id) ON DELETE CASCADE,
        PRIMARY KEY (
            ml_model_id,
            haul_id,
            species_group_id,
            catch_location_id
        )
    );
