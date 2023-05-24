use crate::{
    error::PostgresError,
    models::{Trip, TripAssemblerConflict, TripCalculationTimer, TripDetailed},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Result, ResultExt};
use fiskeridir_rs::Gear;
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::{
    FiskeridirVesselId, HaulId, NewTrip, Ordering, Pagination, PrecisionOutcome, PrecisionStatus,
    TripAssemblerId, TripPrecisionUpdate, Trips, TripsConflictStrategy,
};
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) fn detailed_trips_of_vessel_impl(
        &self,
        id: FiskeridirVesselId,
        pagination: Pagination<Trips>,
        ordering: Ordering,
    ) -> Result<impl Stream<Item = Result<TripDetailed, PostgresError>> + '_, PostgresError> {
        let stream = sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!",
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.start_port_id,
    t.end_port_id,
    t.num_deliveries AS "num_deliveries!",
    t.landing_coverage AS "landing_coverage!",
    t.total_living_weight AS "total_living_weight!",
    t.total_gross_weight AS "total_gross_weight!",
    t.total_product_weight AS "total_product_weight!",
    t.delivery_points AS "delivery_points!",
    t.latest_landing_timestamp,
    t.catches::TEXT AS "catches!",
    t.hauls::TEXT AS "hauls!",
    t.delivery_point_catches::TEXT AS "delivery_point_catches!",
    t.vessel_events::TEXT AS "vessel_events!",
    t.gear_ids AS "gear_ids!: Vec<Gear>"
FROM
    trips_view AS t
WHERE
    t.fiskeridir_vessel_id = $1
ORDER BY
    CASE
        WHEN $2 = 1 THEN t.period
    END ASC,
    CASE
        WHEN $2 = 2 THEN t.period
    END DESC
OFFSET
    $3
LIMIT
    $4
            "#,
            id.0,
            ordering as i32,
            pagination.offset() as i64,
            pagination.limit() as i64
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query));

        Ok(stream)
    }

    pub(crate) async fn all_detailed_trips_of_vessel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<TripDetailed>, PostgresError> {
        sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!",
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.start_port_id,
    t.end_port_id,
    t.num_deliveries AS "num_deliveries!",
    t.landing_coverage AS "landing_coverage!",
    t.total_living_weight AS "total_living_weight!",
    t.total_gross_weight AS "total_gross_weight!",
    t.total_product_weight AS "total_product_weight!",
    t.delivery_points AS "delivery_points!",
    t.latest_landing_timestamp,
    t.catches::TEXT AS "catches!",
    t.hauls::TEXT AS "hauls!",
    t.delivery_point_catches::TEXT AS "delivery_point_catches!",
    t.vessel_events::TEXT AS "vessel_events!",
    t.gear_ids AS "gear_ids!: Vec<Gear>"
FROM
    trips_view AS t
WHERE
    t.fiskeridir_vessel_id = $1
            "#,
            vessel_id.0
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn detailed_trip_of_haul_impl(
        &self,
        haul_id: &HaulId,
    ) -> Result<Option<TripDetailed>, PostgresError> {
        let haul_id_vec = vec![haul_id.0.clone()];
        sqlx::query_as!(
            TripDetailed,
            r#"
SELECT
    t.trip_id AS "trip_id!",
    t.trip_assembler_id AS "trip_assembler_id!: TripAssemblerId",
    t.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    t.period AS "period!",
    t.period_precision,
    t.landing_coverage AS "landing_coverage!",
    t.start_port_id,
    t.end_port_id,
    t.num_deliveries AS "num_deliveries!",
    t.total_living_weight AS "total_living_weight!",
    t.total_gross_weight AS "total_gross_weight!",
    t.total_product_weight AS "total_product_weight!",
    t.delivery_points AS "delivery_points!",
    t.latest_landing_timestamp,
    t.catches::TEXT AS "catches!",
    t.hauls::TEXT AS "hauls!",
    t.delivery_point_catches::TEXT AS "delivery_point_catches!",
    t.vessel_events::TEXT AS "vessel_events!",
    t.gear_ids AS "gear_ids!: Vec<Gear>"
FROM
    trips_view AS t
WHERE
    $1::TEXT[] <@ (t.haul_ids)
            "#,
            haul_id_vec.as_slice()
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn trip_calculation_timers_impl(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripCalculationTimer>, PostgresError> {
        sqlx::query_as!(
            TripCalculationTimer,
            r#"
SELECT
    fiskeridir_vessel_id,
    timer AS "timestamp"
FROM
    trip_calculation_timers
WHERE
    trip_assembler_id = $1
            "#,
            trip_assembler_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn trip_assembler_conflicts(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, PostgresError> {
        sqlx::query_as!(
            TripAssemblerConflict,
            r#"
SELECT
    fiskeridir_vessel_id,
    "conflict" AS "timestamp"
FROM
    trip_assembler_conflicts
WHERE
    trip_assembler_id = $1
            "#,
            trip_assembler_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn trips_of_vessel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
            "#,
            vessel_id.0,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_trips_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        new_trip_calculation_time: DateTime<Utc>,
        conflict_strategy: TripsConflictStrategy,
        trips: Vec<NewTrip>,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<(), PostgresError> {
        let mut period = Vec::with_capacity(trips.len());
        let mut landing_coverage = Vec::with_capacity(trips.len());
        let mut start_port_id = Vec::with_capacity(trips.len());
        let mut end_port_id = Vec::with_capacity(trips.len());
        let mut trip_assembler_ids = Vec::with_capacity(trips.len());
        let mut fiskeridir_vessel_ids = Vec::with_capacity(trips.len());

        let earliest_trip_start = trips[0].period.start();
        for t in trips {
            period.push(
                PgRange::try_from(&t.period)
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            landing_coverage.push(
                PgRange::try_from(&t.landing_coverage)
                    .into_report()
                    .change_context(PostgresError::DataConversion)?,
            );
            start_port_id.push(t.start_port_code);
            end_port_id.push(t.end_port_code);
            trip_assembler_ids.push(trip_assembler_id as i32);
            fiskeridir_vessel_ids.push(vessel_id.0);
        }

        let earliest_trip_period = &period[0];

        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    trip_calculation_timers (fiskeridir_vessel_id, trip_assembler_id, timer)
VALUES
    ($1, $2, $3)
ON CONFLICT (fiskeridir_vessel_id) DO
UPDATE
SET
    timer = excluded.timer
            "#,
            vessel_id.0,
            trip_assembler_id as i32,
            new_trip_calculation_time,
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        match conflict_strategy {
            TripsConflictStrategy::Replace => sqlx::query!(
                r#"
DELETE FROM trips
WHERE
    period && ANY ($1)
    AND fiskeridir_vessel_id = $2
    AND trip_assembler_id = $3
            "#,
                period,
                vessel_id.0,
                trip_assembler_id as i32,
            )
            .execute(&mut tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ()),
            TripsConflictStrategy::Error => Ok(()),
        }?;

        match trip_assembler_id {
            TripAssemblerId::Landings => Ok(()),
            TripAssemblerId::Ers => sqlx::query!(
                r#"
UPDATE trips
SET
    landing_coverage = tstzrange (LOWER(period), $3)
WHERE
    trip_id = (
        SELECT
            trip_id
        FROM
            trips
        WHERE
            fiskeridir_vessel_id = $1
            AND period < $2
        ORDER BY
            period DESC
        LIMIT
            1
    )
            "#,
                vessel_id.0,
                earliest_trip_period,
                earliest_trip_start,
            )
            .execute(&mut tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ()),
        }?;

        sqlx::query!(
            r#"
INSERT INTO
    trips (
        period,
        landing_coverage,
        start_port_id,
        end_port_id,
        trip_assembler_id,
        fiskeridir_vessel_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::tstzrange[],
        $2::tstzrange[],
        $3::VARCHAR[],
        $4::VARCHAR[],
        $5::INT[],
        $6::BIGINT[]
    )
            "#,
            period,
            landing_coverage,
            start_port_id as _,
            end_port_id as _,
            &trip_assembler_ids,
            &fiskeridir_vessel_ids,
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn trip_prior_to_timestamp_exclusive(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND UPPER(period) < $2
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.0,
            time
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn trip_prior_to_timestamp_inclusive(
        &self,
        vessel_id: FiskeridirVesselId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND UPPER(period) <= $2
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.0,
            time
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn update_trip_precisions_impl(
        &self,
        updates: Vec<TripPrecisionUpdate>,
    ) -> Result<(), PostgresError> {
        let len = updates.len();
        let mut trip_id = Vec::with_capacity(len);
        let mut period_precision = Vec::with_capacity(len);
        let mut trip_precision_status_id = Vec::with_capacity(len);
        let mut start_precision_id = Vec::with_capacity(len);
        let mut start_precision_direction = Vec::with_capacity(len);
        let mut end_precision_id = Vec::with_capacity(len);
        let mut end_precision_direction = Vec::with_capacity(len);

        for u in updates {
            trip_id.push(u.trip_id.0);
            match u.outcome {
                PrecisionOutcome::Success {
                    new_period,
                    start_precision,
                    end_precision,
                } => {
                    let pg_range: PgRange<DateTime<Utc>> = PgRange {
                        start: std::ops::Bound::Excluded(new_period.start()),
                        end: std::ops::Bound::Included(new_period.end()),
                    };
                    trip_precision_status_id.push(PrecisionStatus::Successful.name().to_string());
                    period_precision.push(Some(pg_range));
                    start_precision_id.push(start_precision.as_ref().map(|v| v.id as i32));
                    start_precision_direction.push(
                        start_precision
                            .as_ref()
                            .map(|v| v.direction.name().to_string()),
                    );
                    end_precision_id.push(end_precision.as_ref().map(|v| v.id as i32));
                    end_precision_direction
                        .push(end_precision.map(|v| v.direction.name().to_string()));
                }
                PrecisionOutcome::Failed => {
                    trip_precision_status_id.push(PrecisionStatus::Attempted.name().to_string());
                    period_precision.push(None);
                    start_precision_id.push(None);
                    start_precision_direction.push(None);
                    end_precision_id.push(None);
                    end_precision_direction.push(None);
                }
            };
        }

        sqlx::query!(
            r#"
UPDATE trips
SET
    period_precision = u.period_precision,
    trip_precision_status_id = u.precision_status,
    start_precision_id = u.start_precision_id,
    end_precision_id = u.end_precision_id,
    start_precision_direction = u.start_precision_direction,
    end_precision_direction = u.end_precision_direction
FROM
    UNNEST(
        $1::BIGINT[],
        $2::tstzrange[],
        $3::VARCHAR[],
        $4::INT[],
        $5::INT[],
        $6::VARCHAR[],
        $7::VARCHAR[]
    ) u (
        trip_id,
        period_precision,
        precision_status,
        start_precision_id,
        end_precision_id,
        start_precision_direction,
        end_precision_direction
    )
WHERE
    trips.trip_id = u.trip_id
            "#,
            trip_id.as_slice(),
            period_precision.as_slice() as _,
            trip_precision_status_id.as_slice(),
            start_precision_id.as_slice() as _,
            end_precision_id.as_slice() as _,
            start_precision_direction.as_slice() as _,
            end_precision_direction.as_slice() as _,
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub(crate) async fn trips_without_precision_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        assembler_id: TripAssemblerId,
    ) -> Result<Vec<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    period_precision,
    landing_coverage,
    trip_assembler_id AS "trip_assembler_id!: TripAssemblerId"
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
    AND trip_precision_status_id = $3
            "#,
            vessel_id.0,
            assembler_id as i32,
            PrecisionStatus::Unprocessed.name()
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
