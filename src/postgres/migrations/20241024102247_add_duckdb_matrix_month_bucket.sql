TRUNCATE duckdb_data_version;

ALTER TABLE duckdb_data_version
ADD COLUMN matrix_month_bucket int NOT NULL;
