CREATE TABLE ais_data_migration_progress (
    mmsi int NOT NULL references ais_vessels(mmsi),
    progress timestamptz,
    PRIMARY KEY (mmsi)
);

ALTER TABLE ais_positions ALTER COLUMN navigation_status_id DROP NOT NULL;
ALTER TABLE current_ais_positions ALTER COLUMN navigation_status_id DROP NOT NULL;
