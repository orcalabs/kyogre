CREATE TABLE
    duckdb_data_version (
        "version" INT NOT NULL,
        duckdb_data_version_id VARCHAR PRIMARY KEY
    );

INSERT INTO
    duckdb_data_version ("version", duckdb_data_version_id)
VALUES
    (0, 'hauls');

REVOKE DELETE,
TRUNCATE ON public.duckdb_data_version
FROM
    public;
