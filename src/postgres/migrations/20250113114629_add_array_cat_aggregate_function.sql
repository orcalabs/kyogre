CREATE AGGREGATE array_concat (anycompatiblearray) (
    sfunc = ARRAY_CAT,
    stype = anycompatiblearray,
    initcond = '{}'
);
