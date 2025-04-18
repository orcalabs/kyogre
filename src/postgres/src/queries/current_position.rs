use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisVmsPosition, CurrentPosition, CurrentPositionVessel, CurrentPositionsUpdate,
    EarliestVmsUsedBy, FiskeridirVesselId, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY,
    LEISURE_VESSEL_SHIP_TYPES, Mmsi, NavigationStatus, PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
    PositionType, TripPositionLayerId,
};
use sqlx::Postgres;

use crate::{PostgresAdapter, error::Result, models};

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
            CASE
                WHEN $2 = 0 THEN TRUE
                WHEN $2 = 1 THEN (
                    length >= $3
                    AND (
                        ship_type IS NOT NULL
                        AND NOT (ship_type = ANY ($4::INT[]))
                        OR length > $5
                    )
                )
            END
        )
    )
            "#,
            limit,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
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
    0 AS "trip_cumulative_fuel_consumption_liter!",
    0 AS "trip_cumulative_cargo_weight!",
    NULL AS "active_gear?: Gear"
FROM
    current_trip_positions c
    INNER JOIN active_vessels m ON m.fiskeridir_vessel_id = c.fiskeridir_vessel_id
WHERE
    c.fiskeridir_vessel_id = $1::BIGINT
    AND (
        m.mmsi IS NULL
        OR (
            CASE
                WHEN $2 = 0 THEN TRUE
                WHEN $2 = 1 THEN (
                    length >= $3
                    AND (
                        ship_type IS NOT NULL
                        AND NOT (ship_type = ANY ($4::INT[]))
                        OR length > $5
                    )
                )
            END
        )
    )
ORDER BY
    timestamp ASC
            "#,
            vessel_id.into_inner(),
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn current_position_vessels_impl(&self) -> Result<Vec<CurrentPositionVessel>> {
        sqlx::query_as!(
            CurrentPositionVessel,
            r#"
SELECT
    q.fiskeridir_vessel_id AS "id!: FiskeridirVesselId",
    q.mmsi AS "mmsi: Mmsi",
    q.call_sign AS "call_sign: CallSign",
    q.departure_timestamp AS "current_trip_start?",
    CASE
        WHEN q.latest_position IS NULL THEN NULL
        ELSE LEAST(q.latest_position, q.earliest_vms_insertion)
    END AS processing_start
FROM
    (
        SELECT
            f.fiskeridir_vessel_id,
            f.mmsi,
            f.call_sign,
            t.departure_timestamp,
            (
                SELECT
                    MAX(p.timestamp)
                FROM
                    current_trip_positions p
                WHERE
                    p.fiskeridir_vessel_id = f.fiskeridir_vessel_id
            ) AS latest_position,
            (
                SELECT
                    v.timestamp
                FROM
                    earliest_vms_insertion v
                WHERE
                    v.call_sign = f.call_sign
                    AND used_by = $1
            ) AS earliest_vms_insertion
        FROM
            active_vessels f
            LEFT JOIN current_trips t ON f.fiskeridir_vessel_id = t.fiskeridir_vessel_id
        WHERE
            f.mmsi IS NOT NULL
            OR f.call_sign IS NOT NULL
    ) q
            "#,
            EarliestVmsUsedBy::CurrentTripPositionsProcessor as i32,
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
WITH
    vessels AS (
        SELECT
            fiskeridir_vessel_id,
            mmsi,
            call_sign
        FROM
            active_vessels
        WHERE
            fiskeridir_vessel_id = ANY ($1::BIGINT[])
    )
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
    ON (q.fiskeridir_vessel_id) q.fiskeridir_vessel_id,
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
    (
        SELECT
            v.fiskeridir_vessel_id,
            p.mmsi,
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
            $2::INT AS position_type_id
        FROM
            current_ais_positions p
            INNER JOIN vessels v ON p.mmsi = v.mmsi
        UNION ALL
        SELECT
            v.fiskeridir_vessel_id,
            NULL AS mmsi,
            p.call_sign,
            latitude,
            longitude,
            "timestamp",
            course AS course_over_ground,
            speed,
            NULL AS navigation_status_id,
            NULL AS rate_of_turn,
            NULL AS true_heading,
            distance_to_shore,
            $3::INT AS position_type_id
        FROM
            current_vms_positions p
            INNER JOIN vessels v ON p.call_sign = v.call_sign
    ) q
ORDER BY
    q.fiskeridir_vessel_id,
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
            &vessel_ids as &[FiskeridirVesselId],
            PositionType::Ais as i32,
            PositionType::Vms as i32,
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
        let mut call_signs = Vec::with_capacity(len);
        let mut delete_boundaries_lower = Vec::with_capacity(len);
        let mut delete_boundaries_upper = Vec::with_capacity(len);

        for v in updates {
            vessel_ids.push(v.id);
            delete_boundaries_lower.push(v.delete_boundary_lower);
            delete_boundaries_upper.push(v.delete_boundary_upper);
            call_signs.push(v.call_sign.as_deref());
        }

        sqlx::query!(
            r#"
WITH
    inputs AS (
        SELECT
            UNNEST($1::BIGINT[]) AS vessel_id,
            UNNEST($2::TIMESTAMPTZ[]) AS delete_boundary_lower,
            UNNEST($3::TIMESTAMPTZ[]) AS delete_boundary_upper,
            UNNEST($4::TEXT[]) AS call_sign
    ),
    delete_1 AS (
        DELETE FROM current_trip_positions p USING inputs i
        WHERE
            p.fiskeridir_vessel_id = i.vessel_id
            AND p.timestamp < i.delete_boundary_lower
    ),
    delete_2 AS (
        DELETE FROM current_trip_positions p USING inputs i
        WHERE
            p.fiskeridir_vessel_id = i.vessel_id
            AND p.timestamp > i.delete_boundary_upper
    )
DELETE FROM earliest_vms_insertion v USING inputs i
WHERE
    v.call_sign = i.call_sign
    AND v.used_by = $5
    AND (
        v.timestamp <= i.delete_boundary_lower
        OR v.timestamp >= i.delete_boundary_upper
    )
            "#,
            &vessel_ids as &[FiskeridirVesselId],
            &delete_boundaries_lower,
            &delete_boundaries_upper,
            &call_signs as &[Option<&str>],
            EarliestVmsUsedBy::CurrentTripPositionsProcessor as i32,
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
