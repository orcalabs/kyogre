TRUNCATE engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'FuelEstimation'
    OR destination = 'FuelEstimation';

DELETE FROM engine_states
WHERE
    engine_state_id = 'FuelEstimation';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Benchmark', 'HaulDistribution');

CREATE TABLE processors (
    processor_id int PRIMARY KEY,
    description varchar NOT NULL CHECK (description != '')
);

CREATE TABLE processing_runs (
    processor_id int NOT NULL REFERENCES processors (processor_id),
    latest_run timestamptz
);

INSERT INTO
    processors (processor_id, description)
VALUES
    (1, 'fuel_processor')
