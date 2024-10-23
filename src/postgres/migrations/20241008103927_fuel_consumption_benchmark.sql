INSERT INTO
    trip_benchmarks (trip_benchmark_id, description)
VALUES
    (3, 'weight per distance'),
    (4, 'fuel consumption'),
    (5, 'weight per fuel');

CREATE TABLE trip_benchmark_status (
    trip_benchmark_status_id int NOT NULL PRIMARY KEY,
    status text NOT NULL CHECK (status != '') UNIQUE
);

INSERT INTO
    trip_benchmark_status (trip_benchmark_status_id, status)
VALUES
    (1, 'must recompute'),
    (2, 'must refresh'),
    (3, 'refreshed');

ALTER TABLE trip_benchmark_outputs
ADD COLUMN status INT NOT NULL DEFAULT 2 REFERENCES trip_benchmark_status (trip_benchmark_status_id);

ALTER TABLE trips_detailed
ADD COLUMN fuel_consumption DOUBLE PRECISION;
