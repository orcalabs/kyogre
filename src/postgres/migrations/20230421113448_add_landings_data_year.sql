ALTER TABLE landings
ADD COLUMN data_year INT;

CREATE INDEX ON landings (data_year);

DROP TRIGGER landings_after_insert_add_trip_assembler_conflicts ON landings;

CREATE TRIGGER landings_after_insert_or_delete_add_trip_assembler_conflicts
AFTER INSERT
OR DELETE ON landings FOR EACH ROW
EXECUTE FUNCTION add_conflicting_landing ();
