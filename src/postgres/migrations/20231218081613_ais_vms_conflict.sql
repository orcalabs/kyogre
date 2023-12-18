INSERT INTO
    trip_position_layers (trip_position_layer_id, description)
VALUES
    (3, 'ais_vms_conflict');

UPDATE trips
SET
    position_layers_status = 1;
