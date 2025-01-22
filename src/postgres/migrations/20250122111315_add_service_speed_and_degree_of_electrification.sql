ALTER TABLE fiskeridir_vessels
ADD COLUMN service_speed DOUBLE PRECISION CHECK (service_speed > 0.0),
ADD COLUMN degree_of_electrification DOUBLE PRECISION CHECK (
    degree_of_electrification <= 1.0
    AND degree_of_electrification >= 0.0
);
