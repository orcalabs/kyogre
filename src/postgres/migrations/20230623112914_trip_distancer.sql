CREATE TABLE
    trip_distancers (
        trip_distancer_id INT PRIMARY KEY,
        description TEXT NOT NULL
    );

INSERT INTO
    trip_distancers (trip_distancer_id, description)
VALUES
    (1, 'ais_vms');

ALTER TABLE trips
ADD COLUMN distance DECIMAL,
ADD COLUMN distancer_id INT REFERENCES trip_distancers (trip_distancer_id);

INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('TripDistance');

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'HaulDistribution'
    AND destination = 'Pending';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Pending', 'TripDistance'),
    ('HaulDistribution', 'TripDistance'),
    ('TripDistance', 'Pending');
