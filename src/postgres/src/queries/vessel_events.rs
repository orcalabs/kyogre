use crate::{error::Result, models::VesselEventDetailed, PostgresAdapter};
use chrono::{DateTime, Utc};
use futures::{Stream, TryStreamExt};
use kyogre_core::{FiskeridirVesselId, VesselEventType};

impl PostgresAdapter {
    pub(crate) fn all_landing_events(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> impl Stream<Item = Result<VesselEventDetailed>> + '_ {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT
    v.vessel_event_id,
    v.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    v.report_timestamp,
    v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
    NULL AS "port_id",
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    NULL AS "estimated_timestamp: _"
FROM
    vessel_events v
WHERE
    v.fiskeridir_vessel_id = $1::BIGINT
    AND v.vessel_event_type_id = $2
ORDER BY
    v.report_timestamp
           "#,
            vessel_id.into_inner(),
            VesselEventType::Landing as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn all_landing_events_after_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        end: DateTime<Utc>,
    ) -> impl Stream<Item = Result<VesselEventDetailed>> + '_ {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT
    v.vessel_event_id,
    v.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    v.report_timestamp,
    v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
    NULL AS "port_id",
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    NULL AS "estimated_timestamp: _"
FROM
    vessel_events v
WHERE
    v.fiskeridir_vessel_id = $1::BIGINT
    AND v.vessel_event_type_id = $2
    AND v.report_timestamp > $3::TIMESTAMPTZ
ORDER BY
    v.report_timestamp
           "#,
            vessel_id.into_inner(),
            VesselEventType::Landing as i32,
            end,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn landing_trip_start_and_end_events_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> impl Stream<Item = Result<VesselEventDetailed>> + '_ {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT DISTINCT
    ON (v.report_timestamp) v.vessel_event_id,
    v.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    v.report_timestamp,
    v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
    NULL AS "port_id",
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    NULL AS "estimated_timestamp: _"
FROM
    vessel_events v
WHERE
    v.fiskeridir_vessel_id = $1::BIGINT
    AND v.vessel_event_type_id = $2
    AND (
        v.report_timestamp = $3::TIMESTAMPTZ
        OR v.report_timestamp = $4::TIMESTAMPTZ
    )
ORDER BY
    v.report_timestamp
           "#,
            vessel_id.into_inner(),
            VesselEventType::Landing as i32,
            start,
            end,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn all_ers_por_and_dep_events(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> impl Stream<Item = Result<VesselEventDetailed>> + '_ {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT
    vessel_event_id AS "vessel_event_id!",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    report_timestamp AS "report_timestamp!",
    "vessel_event_type_id!: VesselEventType",
    port_id,
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    estimated_timestamp
FROM
    (
        SELECT
            v.vessel_event_id,
            v.fiskeridir_vessel_id,
            v.report_timestamp,
            v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
            d.port_id,
            d.relevant_year,
            d.message_number,
            d.departure_timestamp AS estimated_timestamp
        FROM
            vessel_events v
            INNER JOIN ers_departures d ON d.vessel_event_id = v.vessel_event_id
        WHERE
            v.fiskeridir_vessel_id = $1::BIGINT
            AND v.occurence_timestamp >= '1970-01-01T00:00:00Z'::TIMESTAMPTZ
        UNION
        SELECT
            v.vessel_event_id,
            v.fiskeridir_vessel_id,
            v.report_timestamp,
            v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
            a.port_id,
            a.relevant_year,
            a.message_number,
            a.arrival_timestamp AS estimated_timestamp
        FROM
            vessel_events v
            INNER JOIN ers_arrivals a ON a.vessel_event_id = v.vessel_event_id
        WHERE
            v.fiskeridir_vessel_id = $1::BIGINT
            AND v.occurence_timestamp >= '1970-01-01T00:00:00Z'::TIMESTAMPTZ
    ) q
ORDER BY
    estimated_timestamp,
    relevant_year,
    message_number
           "#,
            vessel_id.into_inner(),
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn all_ers_por_and_dep_events_after_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
    ) -> impl Stream<Item = Result<VesselEventDetailed>> + '_ {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
WITH
    trip_por_message_number AS (
        SELECT
            fiskeridir_vessel_id,
            arrival_timestamp AS occurence_timestamp,
            relevant_year,
            message_number
        FROM
            ers_arrivals
        WHERE
            fiskeridir_vessel_id = $1::bigint
            AND arrival_timestamp = $2::TIMESTAMPTZ
        ORDER BY
            relevant_year,
            message_number
        LIMIT
            1
    )
SELECT
    vessel_event_id AS "vessel_event_id!",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    report_timestamp AS "report_timestamp!",
    vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
    port_id,
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    estimated_timestamp
FROM
    (
        SELECT
            v.vessel_event_id,
            v.fiskeridir_vessel_id,
            v.report_timestamp,
            v.vessel_event_type_id,
            d.port_id,
            d.relevant_year,
            d.message_number,
            d.departure_timestamp AS estimated_timestamp
        FROM
            vessel_events v
            INNER JOIN ers_departures d ON d.vessel_event_id = v.vessel_event_id
            INNER JOIN trip_por_message_number t ON t.fiskeridir_vessel_id = d.fiskeridir_vessel_id
            AND (
                d.departure_timestamp > t.occurence_timestamp
                OR (
                    d.relevant_year > t.relevant_year
                    OR (
                        d.relevant_year = t.relevant_year
                        AND d.message_number > t.message_number
                    )
                )
            )
        WHERE
            v.fiskeridir_vessel_id = $1::BIGINT
            AND v.occurence_timestamp >= $2::TIMESTAMPTZ
        UNION
        SELECT
            v.vessel_event_id,
            v.fiskeridir_vessel_id,
            v.report_timestamp,
            v.vessel_event_type_id,
            a.port_id,
            a.relevant_year,
            a.message_number,
            a.arrival_timestamp AS estimated_timestamp
        FROM
            vessel_events v
            INNER JOIN ers_arrivals a ON a.vessel_event_id = v.vessel_event_id
            INNER JOIN trip_por_message_number t ON t.fiskeridir_vessel_id = a.fiskeridir_vessel_id
            AND (
                a.arrival_timestamp > t.occurence_timestamp
                OR (
                    a.relevant_year > t.relevant_year
                    OR (
                        a.relevant_year = t.relevant_year
                        AND a.message_number > t.message_number
                    )
                )
            )
        WHERE
            v.fiskeridir_vessel_id = $1::BIGINT
            AND v.occurence_timestamp >= $2::TIMESTAMPTZ
    ) q
ORDER BY
    estimated_timestamp,
    relevant_year,
    message_number
           "#,
            vessel_id.into_inner(),
            timestamp,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn ers_trip_start_and_end_events_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> impl Stream<Item = Result<VesselEventDetailed>> + '_ {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT
    vessel_event_id AS "vessel_event_id!",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    report_timestamp AS "report_timestamp!",
    vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
    port_id,
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    estimated_timestamp
FROM
    (
        SELECT DISTINCT
            ON (vessel_event_type_id) *
        FROM
            (
                SELECT
                    v.vessel_event_id,
                    v.fiskeridir_vessel_id,
                    v.report_timestamp,
                    v.vessel_event_type_id,
                    d.port_id,
                    d.relevant_year,
                    d.message_number,
                    d.departure_timestamp AS estimated_timestamp
                FROM
                    vessel_events v
                    INNER JOIN ers_departures d ON d.vessel_event_id = v.vessel_event_id
                WHERE
                    v.fiskeridir_vessel_id = $1::BIGINT
                    AND v.occurence_timestamp = $2::TIMESTAMPTZ
                UNION
                SELECT
                    v.vessel_event_id,
                    v.fiskeridir_vessel_id,
                    v.report_timestamp,
                    v.vessel_event_type_id,
                    a.port_id,
                    a.relevant_year,
                    a.message_number,
                    a.arrival_timestamp AS estimated_timestamp
                FROM
                    vessel_events v
                    INNER JOIN ers_arrivals a ON a.vessel_event_id = v.vessel_event_id
                WHERE
                    v.fiskeridir_vessel_id = $1::BIGINT
                    AND v.occurence_timestamp = $3::TIMESTAMPTZ
            ) q1
        ORDER BY
            vessel_event_type_id,
            estimated_timestamp,
            relevant_year,
            message_number
    ) q
ORDER BY
    estimated_timestamp
           "#,
            vessel_id.into_inner(),
            start,
            end,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn dangling_vessel_events(&self) -> Result<i64> {
        let row = sqlx::query!(
            r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
    LEFT JOIN ers_dca e ON e.vessel_event_id = v.vessel_event_id
    LEFT JOIN ers_departures d ON d.vessel_event_id = v.vessel_event_id
    LEFT JOIN ers_arrivals a ON a.vessel_event_id = v.vessel_event_id
    LEFT JOIN ers_tra t ON t.vessel_event_id = v.vessel_event_id
    LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
WHERE
    l.landing_id IS NULL
    AND e.message_id IS NULL
    AND d.message_id IS NULL
    AND a.message_id IS NULL
    AND t.message_id IS NULL
    AND h.haul_id IS NULL
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.count)
    }
}
