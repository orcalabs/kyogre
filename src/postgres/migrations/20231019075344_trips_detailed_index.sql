DROP INDEX trips_detailed_stop_timestamp_idx;

CREATE INDEX ON trips_detailed (stop_timestamp, start_timestamp);
