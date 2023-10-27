ALTER TABLE trips_detailed
ADD COLUMN cache_version BIGINT NOT NULL DEFAULT 0;

CREATE INDEX ON trips_detailed (cache_version);
