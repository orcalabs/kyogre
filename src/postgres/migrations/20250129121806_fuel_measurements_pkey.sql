ALTER TABLE fuel_measurements
DROP CONSTRAINT fuel_measurements_pkey,
ADD COLUMN fuel_measurements_id BIGSERIAL NOT NULL PRIMARY KEY,
ADD CONSTRAINT fuel_measurements_barentswatch_user_id_call_sign_timestamp_uniq UNIQUE (barentswatch_user_id, call_sign, timestamp);
