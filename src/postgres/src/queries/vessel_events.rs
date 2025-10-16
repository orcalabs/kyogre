use crate::{PostgresAdapter, error::Result, models::VesselEventDetailed};
use futures::{Stream, TryStreamExt};
use kyogre_core::{FiskeridirVesselId, VesselEventType};
use strum::IntoEnumIterator;

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

    pub(crate) async fn dangling_vessel_events(&self) -> Result<i64> {
        let mut count = 0;
        for e in VesselEventType::iter() {
            match e {
                VesselEventType::Landing => {
                    count += sqlx::query!(
                        r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN landings l ON l.vessel_event_id = v.vessel_event_id
WHERE
    l.landing_id IS NULL
    AND v.vessel_event_type_id = $1
            "#,
                        e as i32
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .count;
                }
                VesselEventType::ErsDca => {
                    count += sqlx::query!(
                        r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN ers_dca e ON e.vessel_event_id = v.vessel_event_id
WHERE
    e.message_id IS NULL
    AND v.vessel_event_type_id = $1
            "#,
                        e as i32
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .count;
                }
                VesselEventType::ErsPor => {
                    count += sqlx::query!(
                        r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN ers_arrivals a ON a.vessel_event_id = v.vessel_event_id
WHERE
    a.message_id IS NULL
    AND v.vessel_event_type_id = $1
            "#,
                        e as i32
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .count;
                }
                VesselEventType::ErsDep => {
                    count += sqlx::query!(
                        r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN ers_departures d ON d.vessel_event_id = v.vessel_event_id
WHERE
    d.message_id IS NULL
    AND v.vessel_event_type_id = $1
            "#,
                        e as i32
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .count;
                }
                VesselEventType::ErsTra => {
                    count += sqlx::query!(
                        r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN ers_tra t ON t.vessel_event_id = v.vessel_event_id
WHERE
    t.message_id IS NULL
    AND v.vessel_event_type_id = $1
            "#,
                        e as i32
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .count;
                }
                VesselEventType::Haul => {
                    count += sqlx::query!(
                        r#"
SELECT
    COUNT(*) AS "count!"
FROM
    vessel_events v
    LEFT JOIN hauls h ON h.vessel_event_id = v.vessel_event_id
WHERE
    h.haul_id IS NULL
    AND v.vessel_event_type_id = $1
            "#,
                        e as i32
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .count;
                }
            }
        }

        Ok(count)
    }
}
