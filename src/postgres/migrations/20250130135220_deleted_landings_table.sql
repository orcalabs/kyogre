CREATE TABLE deleted_landing_types (
    deleted_landing_type_id INT NOT NULL PRIMARY KEY,
    description TEXT NOT NULL CHECK (description != '')
);

INSERT INTO
    deleted_landing_types
VALUES
    (1, 'new_version'),
    (2, 'removed_from_dataset');

CREATE TABLE deleted_landings (
    landing_id TEXT NOT NULL,
    fiskeridir_vessel_id BIGINT,
    landing_timestamp TIMESTAMPTZ NOT NULL,
    deleted_landing_type_id INT NOT NULL REFERENCES deleted_landing_types (deleted_landing_type_id)
);
