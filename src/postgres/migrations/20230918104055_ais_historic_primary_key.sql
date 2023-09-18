DELETE FROM ais_vessels_historic a
WHERE
    TO_CHAR(a.mmsi, '9999999999999999') || TO_CHAR(a.message_timestamp, 'YYYY-MM-DD HH24:MM:SS') IN (
        SELECT
            TO_CHAR(b.mmsi, '9999999999999999') || TO_CHAR(b.message_timestamp, 'YYYY-MM-DD HH24:MM:SS')
        FROM
            ais_vessels_historic b
        GROUP BY
            b.mmsi,
            b.message_timestamp
        HAVING
            COUNT(*) > 1
    );

ALTER TABLE ais_vessels_historic
ADD PRIMARY KEY (mmsi, message_timestamp);
