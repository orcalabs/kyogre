DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'Benchmark'
    AND destination = 'HaulDistribution';

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('FuelEstimation');

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Pending', 'FuelEstimation'),
    ('FuelEstimation', 'Pending'),
    ('Benchmark', 'FuelEstimation'),
    ('FuelEstimation', 'HaulDistribution');

CREATE TABLE fuel_estimates (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE CASCADE,
    "date" DATE NOT NULL,
    estimate DOUBLE PRECISION NOT NULL,
    status int NOT NULL DEFAULT 3 REFERENCES processing_status (processing_status_id),
    PRIMARY KEY (fiskeridir_vessel_id, "date")
);

UPDATE trip_benchmark_outputs
SET
    status = 1
WHERE
    trip_benchmark_id >= 4;
