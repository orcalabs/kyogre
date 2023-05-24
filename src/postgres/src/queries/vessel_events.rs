use crate::{error::PostgresError, models::VesselEventDetailed, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{FiskeridirVesselId, QueryRange, VesselEventType};
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) async fn landing_events(
        &self,
        vessel_id: FiskeridirVesselId,
        period: &QueryRange,
    ) -> Result<Vec<VesselEventDetailed>, PostgresError> {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT
    v.vessel_event_id,
    v.fiskeridir_vessel_id,
    v."timestamp",
    v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
    NULL AS "port_id",
    NULL AS "arrival_port_id",
    NULL AS "departure_port_id",
    NULL AS "estimated_timestamp: _"
FROM
    vessel_events v
WHERE
    v.fiskeridir_vessel_id = $1
    AND v.vessel_event_type_id = $2
    AND v."timestamp" <@ $3::tstzrange
ORDER BY
    "timestamp"
           "#,
            &(vessel_id.0 as i32),
            VesselEventType::Landing as i32,
            PgRange::from(period),
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn ers_por_and_dep_events(
        &self,
        vessel_id: FiskeridirVesselId,
        period: &QueryRange,
    ) -> Result<Vec<VesselEventDetailed>, PostgresError> {
        sqlx::query_as!(
            VesselEventDetailed,
            r#"
SELECT
    vessel_event_id AS "vessel_event_id!",
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    "timestamp" AS "timestamp!",
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
            v."timestamp",
            v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
            d.port_id,
            d.relevant_year,
            d.message_number,
            d.departure_timestamp AS estimated_timestamp
        FROM
            vessel_events v
            INNER JOIN ers_departures d ON d.vessel_event_id = v.vessel_event_id
        WHERE
            v.fiskeridir_vessel_id = $1
            AND v."timestamp" <@ $2::tstzrange
        UNION
        SELECT
            v.vessel_event_id,
            v.fiskeridir_vessel_id,
            v."timestamp",
            v.vessel_event_type_id AS "vessel_event_type_id!: VesselEventType",
            a.port_id,
            a.relevant_year,
            a.message_number,
            a.arrival_timestamp AS estimated_timestamp
        FROM
            vessel_events v
            INNER JOIN ers_arrivals a ON a.vessel_event_id = v.vessel_event_id
        WHERE
            v.fiskeridir_vessel_id = $1
            AND v."timestamp" <@ $2::tstzrange
    ) q
ORDER BY
    "timestamp"
           "#,
            &(vessel_id.0 as i32),
            PgRange::from(period),
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
