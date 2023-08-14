INSERT INTO
    engine_states (engine_state_id)
VALUES
    ('VerifyDatabase');

DELETE FROM valid_engine_transitions
WHERE
    source = 'UpdateDatabaseViews'
    AND destination = 'Pending';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('UpdateDatabaseViews', 'VerifyDatabase'),
    ('Pending', 'VerifyDatabase'),
    ('VerifyDatabase', 'Pending');
