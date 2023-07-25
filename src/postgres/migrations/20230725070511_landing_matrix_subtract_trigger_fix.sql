CREATE
OR REPLACE FUNCTION subtract_from_landing_matrix () RETURNS TRIGGER LANGUAGE plpgsql AS $$
    BEGIN
        IF (TG_OP = 'DELETE') THEN
            IF OLD.living_weight IS NOT NULL THEN
                UPDATE landing_matrix
                SET
                    living_weight = landing_matrix.living_weight - OLD.living_weight
                WHERE
                    landing_id = OLD.landing_id
                    AND species_group_id = OLD.species_group_id;
            END IF;
        END IF;
        RETURN NEW;
   END;
$$;
