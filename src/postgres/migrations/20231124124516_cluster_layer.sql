INSERT INTO
    trip_position_layers (trip_position_layer_id, description)
VALUES
    (2, 'cluster');

UPDATE trips
SET
    position_layers_status = 1;
