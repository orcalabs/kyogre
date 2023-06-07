DROP MATERIALIZED VIEW trips_view;

DROP FUNCTION update_database_views;

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'UpdateDatabaseViews'
    OR destination = 'UpdateDatabaseViews';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Benchmark', 'Pending');

DELETE FROM engine_states
WHERE
    engine_state_id = 'UpdateDatabaseViews';

CREATE INDEX ON fishing_facilities USING GIST ("period");
