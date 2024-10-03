DROP TABLE vessel_benchmark_outputs,
vessel_benchmarks;

CREATE TABLE trip_benchmarks (
    trip_benchmark_id INT PRIMARY KEY,
    description VARCHAR NOT NULL
);

INSERT INTO
    trip_benchmarks (trip_benchmark_id, description)
VALUES
    (1, 'weight per hour'),
    (2, 'sustainability');

CREATE TABLE trip_benchmark_outputs (
    trip_id BIGINT NOT NULL REFERENCES trips (trip_id) ON DELETE CASCADE,
    trip_benchmark_id INT NOT NULL REFERENCES trip_benchmarks (trip_benchmark_id),
    output DOUBLE PRECISION NOT NULL,
    unrealistic BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (trip_id, trip_benchmark_id)
);
