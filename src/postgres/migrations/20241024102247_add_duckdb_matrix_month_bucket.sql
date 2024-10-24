DROP TABLE duckdb_data_version;

CREATE TABLE duckdb_data_version (
    "version" int NOT NULL,
    matrix_month_bucket int NOT NULL,
    duckdb_data_version_id text NOT NULL CHECK (duckdb_data_version_id != ''),
    PRIMARY KEY (duckdb_data_version_id, "version")
);
