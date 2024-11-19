use fiskeridir_rs::CallSign;
use fiskeridir_rs::DeliveryPointId;
use futures::{Stream, StreamExt, TryStreamExt};
use kyogre_core::{
    AisVmsPosition, Arrival, DeliveryPoint, Departure, FiskeridirVesselId, Mmsi, NavigationStatus,
    NewVesselConflict, PortDockPoint, PositionType, ProcessingStatus, TripPositionLayerId,
    VesselEventType,
};

use crate::{
    error::Result,
    models::{
        FiskeridirAisVesselCombination, ManualDeliveryPoint, NewDeliveryPointId, Port, Tra,
        TripAssemblerLogEntry, VesselConflictInsert, VmsPosition,
    },
    PostgresAdapter,
};

use super::vms::VmsPositionsArg;

impl PostgresAdapter {
    pub(crate) fn trip_assembler_log_impl(
        &self,
    ) -> impl Stream<Item = Result<TripAssemblerLogEntry>> + '_ {
        sqlx::query_as!(
            TripAssemblerLogEntry,
            r#"
SELECT
    trip_assembler_log_id,
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    calculation_timer_prior,
    calculation_timer_post,
    "conflict",
    conflict_vessel_event_timestamp,
    conflict_vessel_event_id,
    conflict_vessel_event_type_id AS "conflict_vessel_event_type_id: VesselEventType",
    prior_trip_vessel_events::TEXT AS "prior_trip_vessel_events!",
    new_vessel_events::TEXT AS "new_vessel_events!",
    conflict_strategy
FROM
    trip_assembler_logs
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }
    pub(crate) async fn all_ers_tra_impl(&self) -> Result<Vec<Tra>> {
        let tra = sqlx::query_as!(
            Tra,
            r#"
SELECT
    e.fiskeridir_vessel_id as "fiskeridir_vessel_id?: FiskeridirVesselId",
    e.latitude,
    e.longitude,
    e.reloading_timestamp,
    e.message_timestamp,
    e.catches::TEXT as "catches!",
    e.reload_to as "reload_to?: FiskeridirVesselId",
    e.reload_from as "reload_from?: FiskeridirVesselId",
    e.reload_to_call_sign as "reload_to_call_sign?: CallSign",
    e.reload_from_call_sign as "reload_from_call_sign?: CallSign"
FROM
    ers_tra_reloads e
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tra)
    }

    pub(crate) async fn all_ers_departures_impl(&self) -> Result<Vec<Departure>> {
        let dep = sqlx::query_as!(
            Departure,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    departure_timestamp AS "timestamp",
    port_id
FROM
    ers_departures
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(dep)
    }

    pub(crate) async fn all_ers_arrivals_impl(&self) -> Result<Vec<Arrival>> {
        let arrivals = sqlx::query_as!(
            Arrival,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    arrival_timestamp AS "timestamp",
    port_id
FROM
    ers_arrivals
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(arrivals)
    }

    pub(crate) async fn delivery_points_log_impl(&self) -> Result<Vec<serde_json::Value>> {
        Ok(sqlx::query!(
            r#"
SELECT
    TO_JSONB(d.*) AS "json!"
FROM
    delivery_points_log d
            "#,
        )
        .fetch(&self.pool)
        .map_ok(|r| r.json)
        .try_collect()
        .await?)
    }

    pub(crate) fn all_vms_impl(&self) -> impl Stream<Item = Result<VmsPosition>> + '_ {
        self.vms_positions_inner(VmsPositionsArg::All)
    }

    pub(crate) async fn all_ais_vms_impl(&self) -> Result<Vec<AisVmsPosition>> {
        let ais = sqlx::query_as!(
            AisVmsPosition,
            r#"
SELECT
    latitude AS "latitude!",
    longitude AS "longitude!",
    "timestamp" AS "timestamp!",
    course_over_ground,
    speed,
    navigational_status AS "navigational_status: NavigationStatus",
    rate_of_turn,
    true_heading,
    distance_to_shore AS "distance_to_shore!",
    position_type_id AS "position_type!: PositionType",
    NULL AS "pruned_by: TripPositionLayerId",
    NULL AS "trip_cumulative_fuel_consumption!: Option<f64>",
    NULL AS "trip_cumulative_cargo_weight!: Option<f64>"
FROM
    (
        SELECT
            latitude,
            longitude,
            "timestamp",
            course_over_ground,
            speed_over_ground AS speed,
            navigation_status_id AS navigational_status,
            rate_of_turn,
            true_heading,
            distance_to_shore,
            $1::INT AS position_type_id
        FROM
            ais_positions a
        UNION ALL
        SELECT
            latitude,
            longitude,
            "timestamp",
            course AS course_over_ground,
            speed,
            NULL AS navigational_status,
            NULL AS rate_of_turn,
            NULL AS true_heading,
            distance_to_shore,
            $2::INT AS position_type_id
        FROM
            vms_positions v
    ) q
ORDER BY
    "timestamp" ASC
            "#,
            PositionType::Ais as i32,
            PositionType::Vms as i32,
        )
        .fetch_all(self.ais_pool())
        .await?;

        Ok(ais)
    }

    pub(crate) async fn port_impl(&self, port_id: &str) -> Result<Option<Port>> {
        self.ports_inner(Some(port_id)).next().await.transpose()
    }

    pub(crate) async fn delivery_point_impl(
        &self,
        id: &DeliveryPointId,
    ) -> Result<Option<DeliveryPoint>> {
        self.delivery_points_inner(Some(id))
            .next()
            .await
            .transpose()
    }

    pub(crate) async fn dock_points_of_port_impl(
        &self,
        port_id: &str,
    ) -> Result<Vec<PortDockPoint>> {
        self.dock_points_inner(Some(port_id)).await
    }

    pub(crate) async fn manual_conflict_override_impl(
        &self,
        overrides: Vec<NewVesselConflict>,
    ) -> Result<()> {
        let mut mmsi = Vec::with_capacity(overrides.len());
        let mut fiskeridir_vessel_id = Vec::with_capacity(overrides.len());

        overrides.iter().for_each(|v| {
            if let Some(val) = v.mmsi {
                mmsi.push(val);
            }
            fiskeridir_vessel_id.push(v.vessel_id);
        });

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    ais_vessels (mmsi)
SELECT
    *
FROM
    UNNEST($1::INT[])
ON CONFLICT DO NOTHING
            "#,
            &mmsi as &[Mmsi],
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
SELECT
    *
FROM
    UNNEST($1::BIGINT[])
ON CONFLICT DO NOTHING
            "#,
            &fiskeridir_vessel_id as &[FiskeridirVesselId],
        )
        .execute(&mut *tx)
        .await?;

        self.unnest_insert_from::<_, _, VesselConflictInsert>(overrides, &mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn queue_trip_reset_impl(&self) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trip_calculation_timers
SET
    queued_reset = TRUE
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn clear_trip_distancing_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips
SET
    distancer_id = NULL,
    distance = NULL
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn clear_trip_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips
SET
    start_precision_id = NULL,
    start_precision_direction = NULL,
    end_precision_id = NULL,
    end_precision_direction = NULL,
    period_precision = NULL,
    trip_precision_status_id = $1
WHERE
    fiskeridir_vessel_id = $2
            "#,
            ProcessingStatus::Unprocessed as i32,
            vessel_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn add_manual_delivery_points_impl(
        &self,
        values: Vec<kyogre_core::ManualDeliveryPoint>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        self.unnest_insert_from::<_, _, NewDeliveryPointId<'_>>(&values, &mut *tx)
            .await?;
        self.unnest_insert_from::<_, _, ManualDeliveryPoint>(values, &mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_deprecated_delivery_point_impl(
        &self,
        old: DeliveryPointId,
        new: DeliveryPointId,
    ) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    deprecated_delivery_points (old_delivery_point_id, new_delivery_point_id)
VALUES
    ($1, $2)
            "#,
            old.into_inner(),
            new.into_inner(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn single_fiskeridir_ais_vessel_combination(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Option<FiskeridirAisVesselCombination>> {
        self.fiskeridir_ais_vessel_combinations_impl(Some(vessel_id))
            .next()
            .await
            .transpose()
    }
}
