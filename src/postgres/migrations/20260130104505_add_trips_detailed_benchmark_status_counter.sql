ALTER TABLE trips_detailed
ADD COLUMN benchmark_state_counter INT NOT NULL DEFAULT 0;

CREATE OR REPLACE FUNCTION increment_benchmark_state_counter () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'UPDATE') THEN
            IF NEW.benchmark_status != OLD.benchmark_status THEN
                NEW.benchmark_state_counter = OLD.benchmark_state_counter + 1;
            END IF;
        END IF;
        RETURN NEW;
    END;
$$;

CREATE TRIGGER trips_detailed_after_update_increment_benchmark_state_counter BEFORE
UPDATE ON trips_detailed FOR EACH ROW
EXECUTE FUNCTION increment_benchmark_state_counter ();

DELETE FROM engine_transitions;

DELETE FROM valid_engine_transitions
WHERE
    source = 'Benchmark'
    OR destination = 'Benchmark';

INSERT INTO
    valid_engine_transitions (source, destination)
VALUES
    ('Trips', 'HaulDistribution');
