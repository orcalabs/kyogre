CREATE
OR REPLACE FUNCTION TO_TSTZRANGE (t1 TIMESTAMPTZ, t2 TIMESTAMPTZ) RETURNS TSTZRANGE LANGUAGE PLPGSQL IMMUTABLE AS $$
    BEGIN
        RETURN CASE
            WHEN t2 < t1 THEN NULL
            ELSE TSTZRANGE (t1, t2, '[]')
        END;
    END;
$$;

ALTER TABLE fishing_facilities
ADD COLUMN fiskeridir_vessel_id BIGINT REFERENCES fiskeridir_vessels (fiskeridir_vessel_id),
ADD COLUMN period TSTZRANGE GENERATED ALWAYS AS (TO_TSTZRANGE (setup_timestamp, removed_timestamp)) STORED;

CREATE INDEX ON fishing_facilities (fiskeridir_vessel_id, "period");

CREATE TABLE
    fishing_facilities_copy AS TABLE fishing_facilities;

DELETE FROM fishing_facilities;

INSERT INTO
    fishing_facilities (
        tool_id,
        barentswatch_vessel_id,
        fiskeridir_vessel_id,
        vessel_name,
        call_sign,
        mmsi,
        imo,
        reg_num,
        sbr_reg_num,
        contact_phone,
        contact_email,
        tool_type,
        tool_type_name,
        tool_color,
        tool_count,
        setup_timestamp,
        setup_processed_timestamp,
        removed_timestamp,
        removed_processed_timestamp,
        last_changed,
        source,
        "comment",
        geometry_wkt,
        api_source
    )
SELECT
    f.tool_id,
    f.barentswatch_vessel_id,
    v.fiskeridir_vessel_id,
    f.vessel_name,
    f.call_sign,
    f.mmsi,
    f.imo,
    f.reg_num,
    f.sbr_reg_num,
    f.contact_phone,
    f.contact_email,
    f.tool_type,
    f.tool_type_name,
    f.tool_color,
    f.tool_count,
    f.setup_timestamp,
    f.setup_processed_timestamp,
    f.removed_timestamp,
    f.removed_processed_timestamp,
    f.last_changed,
    f.source,
    f.comment,
    f.geometry_wkt,
    f.api_source
FROM
    fishing_facilities_copy f
    LEFT JOIN (
        SELECT
            call_sign,
            MIN(fiskeridir_vessel_id) AS fiskeridir_vessel_id
        FROM
            fiskeridir_vessels
        GROUP BY
            call_sign
        HAVING
            COUNT(fiskeridir_vessel_id) = 1
    ) v ON v.call_sign = f.call_sign;

DROP TABLE fishing_facilities_copy;
