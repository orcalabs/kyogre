ALTER TABLE ers_departure_catches
DROP CONSTRAINT ers_departure_catches_message_id_fkey;

ALTER TABLE ers_departure_catches
ADD CONSTRAINT ers_departure_catches_message_id_fkey FOREIGN KEY (message_id) REFERENCES ers_departures (message_id) ON DELETE CASCADE;

ALTER TABLE ers_arrival_catches
DROP CONSTRAINT ers_arrival_catches_message_id_fkey;

ALTER TABLE ers_arrival_catches
ADD CONSTRAINT ers_arrival_catches_message_id_fkey FOREIGN KEY (message_id) REFERENCES ers_arrivals (message_id) ON DELETE CASCADE;

CREATE INDEX ON ers_arrival_catches (message_id);

CREATE INDEX ON ers_departure_catches (message_id);
