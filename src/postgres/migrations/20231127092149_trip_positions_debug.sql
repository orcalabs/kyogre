ALTER TABLE trip_positions
ADD COLUMN pruned_by INT REFERENCES trip_position_layers (trip_position_layer_id);

CREATE TABLE
    trip_positions_pruned (
        trip_id BIGINT NOT NULL REFERENCES trips (trip_id) ON DELETE CASCADE,
        positions JSONB NOT NULL,
        "value" JSONB NOT NULL,
        trip_position_layer_id INT NOT NULL REFERENCES trip_position_layers (trip_position_layer_id)
    );

UPDATE trips
SET
    position_layers_status = 1;
