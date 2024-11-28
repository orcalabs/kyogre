ALTER TABLE trips_detailed
ADD COLUMN has_track BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE trips_detailed t
SET
    has_track = EXISTS (
        SELECT
            1
        FROM
            trip_positions p
        WHERE
            p.trip_id = t.trip_id
    );
