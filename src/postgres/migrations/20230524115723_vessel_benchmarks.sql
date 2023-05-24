CREATE TABLE
    vessel_benchmarks (
        vessel_benchmark_id INT PRIMARY KEY,
        description VARCHAR NOT NULL
    );

INSERT INTO
    vessel_benchmarks (vessel_benchmark_id, description)
VALUES
    (1, 'weight per hour');

CREATE TABLE
    vessel_benchmark_outputs (
        fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
        vessel_benchmark_id INT NOT NULL REFERENCES vessel_benchmarks (vessel_benchmark_id),
        output DECIMAL NOT NULL,
        PRIMARY KEY (fiskeridir_vessel_id, vessel_benchmark_id)
    );

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('Benchmark');

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'TripsPrecision'
    AND destination = 'UpdateDatabaseViews';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('TripsPrecision', 'Benchmark'),
    ('Benchmark', 'UpdateDatabaseViews');
