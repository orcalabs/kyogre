CREATE TABLE fuel_measurements (
    barentswatch_user_id UUID NOT NULL,
    call_sign TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    fuel DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (barentswatch_user_id, call_sign, timestamp)
);
