ALTER TABLE earliest_vms_insertion
DROP CONSTRAINT earliest_vms_insertion_pkey,
ADD COLUMN used_by INT;

UPDATE earliest_vms_insertion
SET
    used_by = 1;

ALTER TABLE earliest_vms_insertion
ALTER COLUMN used_by
SET NOT NULL;

ALTER TABLE earliest_vms_insertion
ADD PRIMARY KEY (call_sign, used_by);
