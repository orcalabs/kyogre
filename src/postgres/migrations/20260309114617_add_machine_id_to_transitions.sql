DELETE FROM engine_transitions;

ALTER TABLE engine_transitions
ADD COLUMN machine_id INT NOT NULL;
