ALTER TABLE fiskeridir_vessels
ADD COLUMN deprecated BOOL,
ADD COLUMN register_landing_reset BOOL;

UPDATE fiskeridir_vessels
SET
    deprecated = FALSE,
    register_landing_reset = FALSE;

ALTER TABLE fiskeridir_vessels
ALTER COLUMN deprecated
SET NOT NULL,
ALTER COLUMN register_landing_reset
SET NOT NULL;
