CREATE TABLE vessel_permissions (
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    permission_type TEXT NOT NULL CHECK (permission_type != ''),
    PRIMARY KEY (fiskeridir_vessel_id, permission_type)
);

CREATE INDEX ON vessel_permissions (permission_type);
