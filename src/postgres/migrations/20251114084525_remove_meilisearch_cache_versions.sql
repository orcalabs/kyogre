ALTER TABLE hauls
DROP COLUMN cache_version;

ALTER TABLE trips_detailed
DROP COLUMN cache_version;

DROP TRIGGER hauls_before_update_increment_cache_version ON hauls;

DROP TRIGGER trips_detailed_before_update_increment_cache_version ON trips_detailed;

DROP FUNCTION hauls_increment_cache_version;

DROP FUNCTION trips_detailed_increment_cache_version;
