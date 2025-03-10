CREATE OR REPLACE FUNCTION public.add_ais_position_partition () RETURNS trigger LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'INSERT') THEN
            EXECUTE FORMAT(
                $f$
                    CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.ais_positions FOR VALUES IN (%L);
                $f$,
                CONCAT('ais_positions_', NEW.mmsi),
                NEW.mmsi
            );
        END IF;

        RETURN NEW;
   END;
$$;
