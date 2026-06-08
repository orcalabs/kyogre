ALTER TABLE user_hauls
ADD COLUMN haul_id BIGINT REFERENCES hauls (haul_id) ON DELETE SET NULL UNIQUE,
ADD COLUMN trip_id BIGINT REFERENCES trips (trip_id) ON DELETE SET NULL CHECK (
    (
        haul_id IS NOT NULL
        AND trip_id IS NULL
    )
    OR (
        haul_id IS NULL
        OR trip_id IS NOT NULL
    )
    OR (
        haul_id IS NULL
        AND trip_id IS NULL
    )
),
ADD CONSTRAINT start_greater_than_end CHECK (
    end_ts IS NULL
    OR (start_ts < end_ts)
),
ALTER COLUMN fiskeridir_vessel_id TYPE BIGINT;

CREATE TABLE user_hauls_refresh_boundary (
    fiskeridir_vessel_id BIGINT PRIMARY KEY REFERENCES fiskeridir_vessels (fiskeridir_vessel_id) ON DELETE CASCADE,
    refresh_boundary TIMESTAMPTZ,
    current_trip_refresh_boundary TIMESTAMPTZ
);

INSERT INTO
    user_hauls_refresh_boundary (
        fiskeridir_vessel_id,
        refresh_boundary,
        current_trip_refresh_boundary
    )
SELECT
    fiskeridir_vessel_id,
    MIN(start_ts),
    MAX(start_ts)
FROM
    user_hauls
GROUP BY
    fiskeridir_vessel_id;

ALTER TABLE trips_detailed
RENAME COLUMN hauls TO ers_hauls;

ALTER TABLE trips_detailed
ADD COLUMN ers_and_user_hauls JSONB NOT NULL DEFAULT '[]'::JSONB;

ALTER TABLE current_trips
RENAME COLUMN hauls TO ers_hauls;

ALTER TABLE current_trips
ADD COLUMN ers_and_user_hauls JSONB NOT NULL DEFAULT '[]'::JSONB;
