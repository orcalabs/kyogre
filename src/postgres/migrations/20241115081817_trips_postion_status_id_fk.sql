ALTER TABLE trips
ALTER COLUMN trip_precision_status_id
DROP DEFAULT;

ALTER TABLE trips
DROP CONSTRAINT trips_trip_precision_status_id_fkey;

ALTER TABLE trips
ALTER COLUMN trip_precision_status_id TYPE int USING (
    CASE trip_precision_status_id
        WHEN 'unprocessed' THEN 1
        WHEN 'attempted' THEN 2
        WHEN 'successful' THEN 3
    END
);

ALTER TABLE trips
ALTER COLUMN trip_precision_status_id
SET DEFAULT 1;

ALTER TABLE trips
ADD CONSTRAINT trips_trip_precision_processing_status_fk FOREIGN KEY (trip_precision_status_id) REFERENCES processing_status (processing_status_id);

DROP TABLE trip_precision_status;
