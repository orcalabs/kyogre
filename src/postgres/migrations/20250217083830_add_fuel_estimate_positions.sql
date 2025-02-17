ALTER TABLE fuel_estimates
ADD COLUMN num_ais_positions int,
ADD COLUMN num_vms_positions int;

UPDATE fuel_estimates
SET
    num_ais_positions = 0,
    num_vms_positions = 0;

ALTER TABLE fuel_estimates
ALTER COLUMN num_ais_positions
SET NOT NULL,
ALTER COLUMN num_vms_positions
SET NOT NULL;

UPDATE fuel_estimates
SET
    status = 1;
