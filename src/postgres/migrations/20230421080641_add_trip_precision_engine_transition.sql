INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('TripsPrecision');

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Pending', 'TripsPrecision'),
    ('Trips', 'TripsPrecision'),
    ('TripsPrecision', 'UpdateDatabaseViews');

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'Trips'
    AND destination = 'UpdateDatabaseViews';
