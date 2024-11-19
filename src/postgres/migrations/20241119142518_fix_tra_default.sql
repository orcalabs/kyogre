UPDATE trips_detailed
SET
    tra = '[]'
WHERE
    tra = '{}';

ALTER TABLE trips_detailed
ALTER COLUMN tra
SET DEFAULT '[]';
