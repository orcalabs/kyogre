CREATE TABLE orgs (
    org_id BIGINT PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type != ''),
    city TEXT CHECK (city != ''),
    name TEXT NOT NULL CHECK (name != ''),
    postal_code INT NOT NULL CHECK (postal_code > 0)
);

CREATE TABLE orgs__fiskeridir_vessels (
    org_id BIGINT REFERENCES orgs (org_id) ON DELETE CASCADE,
    fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE CASCADE,
    PRIMARY KEY (org_id, fiskeridir_vessel_id)
);

CREATE INDEX ON orgs__fiskeridir_vessels (fiskeridir_vessel_id);

UPDATE fiskeridir_vessels
SET
    owners = '[]'
WHERE
    owners IS NULL;

ALTER TABLE fiskeridir_vessels
ALTER COLUMN owners
SET DEFAULT '[]';

ALTER TABLE fiskeridir_vessels
ALTER COLUMN owners
SET NOT NULL;
