CREATE OR REPLACE FUNCTION has_unique_elements (ANYARRAY) RETURNS boolean AS $$
  SELECT COALESCE(
    (SELECT COUNT(DISTINCT x) FROM UNNEST($1) x) = CARDINALITY($1),
    true
  );
$$ LANGUAGE sql IMMUTABLE;

ALTER TABLE fiskeridir_vessels
ADD CONSTRAINT unique_gear_groups CHECK (HAS_UNIQUE_ELEMENTS (gear_group_ids)),
ADD CONSTRAINT unique_species_groups CHECK (HAS_UNIQUE_ELEMENTS (species_group_ids));
