ALTER TABLE vms_positions
ALTER COLUMN course
DROP NOT NULL,
ALTER COLUMN gross_tonnage
DROP NOT NULL,
ALTER COLUMN speed
DROP NOT NULL;
