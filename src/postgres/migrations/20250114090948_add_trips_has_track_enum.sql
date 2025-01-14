CREATE TABLE has_track (
    has_track_id int PRIMARY KEY,
    description text NOT NULL CHECK (description != '')
);

INSERT INTO
    has_track (has_track_id, description)
VALUES
    (1, 'NoTrack'),
    (2, 'TrackUnder15'),
    (3, 'TrackOver15');

ALTER TABLE trips_detailed
ADD COLUMN has_track2 int REFERENCES has_track (has_track_id) ON DELETE cascade;

UPDATE trips_detailed
SET
    has_track2 = CASE
        WHEN trips_detailed.has_track
        AND trips_detailed.fiskeridir_length_group_id <= 2 THEN 2
        WHEN trips_detailed.has_track
        AND trips_detailed.fiskeridir_length_group_id > 2 THEN 3
        ELSE 1
    END;

ALTER TABLE trips_detailed
DROP COLUMN has_track;

ALTER TABLE trips_detailed
RENAME COLUMN has_track2 TO has_track;

ALTER TABLE trips_detailed
ALTER COLUMN has_track
SET NOT NULL;

CREATE INDEX ON trips_detailed (has_track);
