CREATE EXTENSION IF NOT EXISTS intarray;

TRUNCATE ais_area;

ALTER TABLE ais_area
ADD COLUMN mmsis INT[] NOT NULL DEFAULT '{}';

CREATE AGGREGATE INTARRAY_UNION_AGG (INT[]) (
    sfunc = _INT_UNION,
    stype = INT[],
    initcond = '{}'
);
