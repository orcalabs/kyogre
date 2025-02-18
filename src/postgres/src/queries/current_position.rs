use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisVmsPosition, CurrentPosition, CurrentPositionVessel, CurrentPositionsUpdate,
    FiskeridirVesselId, Mmsi, NavigationStatus, PositionType, TripPositionLayerId,
    LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES,
    PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};
use sqlx::Postgres;

use crate::{error::Result, models, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) fn current_positions_impl(
        &self,
        limit: Option<DateTime<Utc>>,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<CurrentPosition>> + '_ {
        sqlx::query_as!(
            CurrentPosition,
            r#"
SELECT
    c.fiskeridir_vessel_id AS "vessel_id!: FiskeridirVesselId",
    c.latitude,
    c.longitude,
    c.timestamp,
    c.course_over_ground,
    c.navigation_status_id AS "navigational_status: NavigationStatus",
    c.rate_of_turn,
    c.speed,
    c.true_heading,
    c.distance_to_shore,
    c.position_type_id AS "position_type!: PositionType"
FROM
    current_positions c
    INNER JOIN active_vessels m ON m.fiskeridir_vessel_id = c.fiskeridir_vessel_id
WHERE
    (
        $1::TIMESTAMPTZ IS NULL
        OR c.timestamp > $1
    )
    AND (
        m.mmsi IS NULL
        OR (
            m.ship_type IS NOT NULL
            AND NOT (m.ship_type = ANY ($2::INT[]))
            OR m.length > $3
        )
    )
    AND (
        CASE
            WHEN $4 = 0 THEN TRUE
            WHEN $4 = 1 THEN m.length >= $5
        END
    )
            "#,
            limit,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) fn current_trip_positions_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisVmsPosition>> + '_ {
        sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    c.latitude,
    c.longitude,
    c.timestamp,
    c.course_over_ground,
    c.navigation_status_id AS "navigational_status: NavigationStatus",
    c.rate_of_turn,
    c.speed,
    c.true_heading,
    c.distance_to_shore,
    c.position_type_id AS "position_type!: PositionType",
    NULL AS "pruned_by: TripPositionLayerId",
    NULL::DOUBLE PRECISION AS trip_cumulative_fuel_consumption_liter,
    NULL::DOUBLE PRECISION AS trip_cumulative_cargo_weight
FROM
    current_trip_positions c
    INNER JOIN active_vessels m ON m.fiskeridir_vessel_id = c.fiskeridir_vessel_id
WHERE
    c.fiskeridir_vessel_id = $1::BIGINT
    AND (
        m.mmsi IS NULL
        OR (
            m.ship_type IS NOT NULL
            AND NOT (m.ship_type = ANY ($2::INT[]))
            OR m.length > $3
        )
    )
    AND (
        CASE
            WHEN $4 = 0 THEN TRUE
            WHEN $4 = 1 THEN m.length >= $5
        END
    )
ORDER BY
    timestamp ASC
            "#,
            vessel_id.into_inner(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn current_position_vessels_impl(&self) -> Result<Vec<CurrentPositionVessel>> {
        sqlx::query_as!(
            CurrentPositionVessel,
            r#"
SELECT
    f.fiskeridir_vessel_id AS "id!: FiskeridirVesselId",
    f.mmsi AS "mmsi: Mmsi",
    f.call_sign AS "call_sign: CallSign",
    -- Hacky fix because sqlx prepare/check flakes on nullability
    COALESCE(t.departure_timestamp, NULL) AS current_trip_start,
    (
        SELECT
            MAX(p.timestamp)
        FROM
            current_trip_positions p
        WHERE
            p.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    ) AS latest_position
FROM
    active_vessels f
    LEFT JOIN current_trips t ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
WHERE
    f.mmsi IS NOT NULL
    OR f.call_sign IS NOT NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.into())
    }

    pub(crate) async fn update_current_positions_impl(
        &self,
        updates: &[CurrentPositionsUpdate],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        self.refresh_current_positions(updates, &mut tx).await?;
        self.delete_current_trip_positions(updates, &mut tx).await?;
        self.add_current_trip_positions(updates, &mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    async fn refresh_current_positions(
        &self,
        updates: &[CurrentPositionsUpdate],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<()> {
        let vessel_ids = updates.iter().map(|v| v.id).collect::<Vec<_>>();

        sqlx::query!(
            r#"
INSERT INTO
    current_positions (
        fiskeridir_vessel_id,
        latitude,
        longitude,
        "timestamp",
        course_over_ground,
        speed,
        navigation_status_id,
        rate_of_turn,
        true_heading,
        distance_to_shore,
        position_type_id
    )
SELECT DISTINCT
    ON (m.fiskeridir_vessel_id) m.fiskeridir_vessel_id,
    q.latitude,
    q.longitude,
    q.timestamp,
    q.course_over_ground,
    q.speed,
    q.navigation_status_id,
    q.rate_of_turn,
    q.true_heading,
    q.distance_to_shore,
    q.position_type_id
FROM
    active_vessels m
    INNER JOIN (
        SELECT
            mmsi,
            NULL AS call_sign,
            latitude,
            longitude,
            "timestamp",
            course_over_ground,
            speed_over_ground AS speed,
            navigation_status_id,
            rate_of_turn,
            true_heading,
            distance_to_shore,
            $1::INT AS position_type_id
        FROM
            current_ais_positions
        UNION ALL
        SELECT
            NULL AS mmsi,
            call_sign,
            latitude,
            longitude,
            "timestamp",
            course AS course_over_ground,
            speed,
            NULL AS navigation_status_id,
            NULL AS rate_of_turn,
            NULL AS true_heading,
            distance_to_shore,
            $2::INT AS position_type_id
        FROM
            current_vms_positions
    ) q ON q.mmsi = m.mmsi
    OR q.call_sign = m.call_sign
WHERE
    m.fiskeridir_vessel_id = ANY ($3::BIGINT[])
ORDER BY
    m.fiskeridir_vessel_id,
    q.timestamp DESC
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    latitude = EXCLUDED.latitude,
    longitude = EXCLUDED.longitude,
    timestamp = EXCLUDED.timestamp,
    course_over_ground = EXCLUDED.course_over_ground,
    speed = EXCLUDED.speed,
    navigation_status_id = EXCLUDED.navigation_status_id,
    rate_of_turn = EXCLUDED.rate_of_turn,
    true_heading = EXCLUDED.true_heading,
    distance_to_shore = EXCLUDED.distance_to_shore,
    position_type_id = EXCLUDED.position_type_id
            "#,
            PositionType::Ais as i32,
            PositionType::Vms as i32,
            &vessel_ids as &[FiskeridirVesselId],
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn delete_current_trip_positions(
        &self,
        updates: &[CurrentPositionsUpdate],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<()> {
        let len = updates.len();
        let mut vessel_ids = Vec::with_capacity(len);
        let mut delete_boundaries = Vec::with_capacity(len);

        for v in updates {
            vessel_ids.push(v.id);
            delete_boundaries.push(v.delete_boundary);
        }

        sqlx::query!(
            r#"
DELETE FROM current_trip_positions p USING (
    SELECT
        UNNEST($1::BIGINT[]) AS vessel_id,
        UNNEST($2::TIMESTAMPTZ[]) AS delete_boundary
) q
WHERE
    p.fiskeridir_vessel_id = q.vessel_id
    AND p.timestamp < q.delete_boundary
            "#,
            &vessel_ids as &[FiskeridirVesselId],
            &delete_boundaries,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn add_current_trip_positions(
        &self,
        updates: &[CurrentPositionsUpdate],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<()> {
        self.unnest_insert_from::<_, _, models::CurrentPosition>(
            updates.iter().flat_map(|v| &v.positions),
            &mut **tx,
        )
        .await
    }
}
