INSERT INTO
    landing_methods (landing_method_id, name)
VALUES
    (16, 'Levering fra annet enn fartøy');

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'HaulWeather'
    OR source = 'DailyWeather'
    OR destination = 'HaulWeather'
    OR destination = 'DailyWeather';

DELETE FROM engine_states
WHERE
    engine_state_id = 'HaulWeather'
    OR engine_state_id = 'DailyWeather';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Scrape', 'Trips'),
    ('HaulDistribution', 'VerifyDatabase');
