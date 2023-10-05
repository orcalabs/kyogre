CREATE TABLE trip_clusters(
    trip_id bigserial REFERENCES trips(trip_id),
    timestamp_start timestamptz NOT NULL,
    timestamp_end timestamptz NOT NULL,
    latitude DECIMAL NOT NULL,
    longitude DECIMAL NOT NULL,
    covariance jsonb
);
