CREATE TABLE trip_fuel (
    trip_fuel_id SERIAL PRIMARY KEY,
    trip_id BIGINT REFERENCES trips (trip_id) ON DELETE SET NULL,
    fuel_liter_used DOUBLE PRECISION NOT NULL,
    fiskeridir_vessel_id BIGINT NOT NULL REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
    barentswatch_user_id UUID NOT NULL,
    "period" TSTZRANGE NOT NULL,
    UNIQUE (trip_id)
);

ALTER TABLE trips_detailed
ADD COLUMN trip_fuel_liter_used DOUBLE PRECISION;
