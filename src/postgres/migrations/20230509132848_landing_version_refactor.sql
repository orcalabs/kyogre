ALTER TABLE landings
DROP CONSTRAINT landings_landing_id_version_key;

CREATE
OR REPLACE FUNCTION public.check_landing_version () RETURNS TRIGGER LANGUAGE plpgsql AS $$
	    DECLARE _current_version_number int;
	    BEGIN
	        IF (TG_OP = 'INSERT') THEN
	            SELECT "version" from landings INTO _current_version_number WHERE landing_id = NEW.landing_id;
	            IF _current_version_number IS NOT NULL THEN
	                IF _current_version_number < NEW.version THEN
	                    DELETE FROM landing_entries
	                    WHERE landing_id = NEW.landing_id;
	                    DELETE FROM landings
	                    WHERE landing_id = NEW.landing_id;
	                    RETURN NEW;
	                ELSIF _current_version_number = NEW.version THEN
	                    RETURN NULL;
	                ELSIF _current_version_number > NEW.version THEN
	                    RETURN NULL;
	                END IF;
	            ELSE
	                RETURN NEW;
	            END IF;
	        END IF;

	        RETURN NEW;
	    END;
	$$;
