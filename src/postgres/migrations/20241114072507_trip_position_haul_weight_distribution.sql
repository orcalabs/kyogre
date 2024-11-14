ALTER TABLE trips
ADD COLUMN trip_position_haul_weight_distribution_status int NOT NULL DEFAULT 1 REFERENCES processing_status (processing_status_id);

ALTER TABLE trip_positions
ADD COLUMN trip_cumulative_haul_weight double precision;
