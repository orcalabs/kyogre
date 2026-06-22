ALTER TABLE user_hauls
ADD COLUMN distance DOUBLE PRECISION,
ADD COLUMN start_latitude DOUBLE PRECISION,
ADD COLUMN start_longitude DOUBLE PRECISION,
ADD COLUMN distance_processing_status INT NOT NULL DEFAULT 1 REFERENCES processing_status (processing_status_id);
