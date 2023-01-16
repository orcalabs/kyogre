CREATE TABLE engine_states (
    engine_state_id varchar NOT NULL,
    PRIMARY KEY (engine_state_id)
);

CREATE TABLE valid_engine_transitions (
    source varchar NOT NULL references engine_states(engine_state_id),
    destination varchar NOT NULL references engine_states(engine_state_id),
    PRIMARY KEY (source, destination)
);

CREATE TABLE engine_transitions (
    engine_transition_id SERIAL NOT NULL,
    transition_date timestamptz NOT NULL,
    source varchar NOT NULL references engine_states(engine_state_id),
    destination varchar NOT NULL references engine_states(engine_state_id),
    FOREIGN KEY(source, destination) references valid_engine_transitions(source, destination),
    PRIMARY KEY(engine_transition_id)
);

INSERT INTO engine_states(engine_state_id) VALUES ('Pending'), ('Sleep'), ('Scrape');
INSERT INTO valid_engine_transitions(source, destination) VALUES ('Pending', 'Sleep');
INSERT INTO valid_engine_transitions(source, destination) VALUES ('Pending', 'Scrape');
INSERT INTO valid_engine_transitions(source, destination) VALUES ('Sleep', 'Pending');
INSERT INTO valid_engine_transitions(source, destination) VALUES ('Scrape', 'Pending');

