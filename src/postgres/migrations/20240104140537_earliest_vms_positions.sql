CREATE TABLE
    earliest_vms_insertion (
        call_sign VARCHAR PRIMARY KEY,
        "timestamp" timestamptz NOT NULL
    );

UPDATE trips
SET
    trip_precision_status_id = 'unprocessed',
    distancer_id = NULL,
    position_layers_status = 1;
