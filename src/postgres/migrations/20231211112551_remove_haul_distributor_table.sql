ALTER TABLE hauls_matrix
ADD COLUMN haul_distribution_status INT NOT NULL DEFAULT (1) REFERENCES processing_status (processing_status_id);

UPDATE hauls_matrix
SET
    haul_distribution_status = 3
WHERE
    haul_distributor_id IS NOT NULL;

ALTER TABLE hauls_matrix
DROP COLUMN haul_distributor_id;

DROP TABLE haul_distributors;
