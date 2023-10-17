INSERT INTO
    trip_precision_types (trip_precision_type_id, "name")
VALUES
    (5, 'distance_to_shore');

UPDATE trips
SET
    trip_precision_status_id = 'unprocessed';
