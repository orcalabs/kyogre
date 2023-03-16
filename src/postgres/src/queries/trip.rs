use crate::{
    error::PostgresError,
    models::{Trip, TripAssemblerConflict, TripCalculationTimer},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{FiskeridirVesselId, NewTrip, TripAssemblerId, TripsConflictStrategy};
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) async fn trip_of_haul_impl(
        &self,
        haul_id: &str,
    ) -> Result<Option<Trip>, PostgresError> {
        let mut trips = sqlx::query_as!(
            Trip,
            r#"
SELECT
    t.trip_id,
    t.period,
    t.landing_coverage,
    t.trip_assembler_id
FROM
    trips AS t
    INNER JOIN hauls_view AS h ON h.period <@ t.period
    AND h.fiskeridir_vessel_id = t.fiskeridir_vessel_id
WHERE
    h.haul_id = $1
            "#,
            haul_id,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        match trips.len() {
            0 => Ok(None),
            1 => Ok(trips.pop()),
            _ => Ok(trips
                .into_iter()
                .find(|t| t.trip_assembler_id == TripAssemblerId::Ers as i32)),
        }
    }
    pub(crate) async fn trip_at_or_prior_to_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    landing_coverage,
    trip_assembler_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
    AND (
        UPPER(period) <= $3
        OR period @> $3
    )
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.0,
            trip_assembler_id as i32,
            time
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn most_recent_trip_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Option<Trip>, PostgresError> {
        sqlx::query_as!(
            Trip,
            r#"
SELECT
    trip_id,
    period,
    landing_coverage,
    trip_assembler_id
FROM
    trips
WHERE
    fiskeridir_vessel_id = $1
    AND trip_assembler_id = $2
ORDER BY
    period DESC
LIMIT
    1
            "#,
            vessel_id.0,
            trip_assembler_id as i32
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
    landing_coverage,
    trip_assembler_id
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
        let mut range = Vec::with_capacity(trips.len());
        let mut start_port_id = Vec::with_capacity(trips.len());
        let mut end_port_id = Vec::with_capacity(trips.len());
        let mut trip_assembler_ids = Vec::with_capacity(trips.len());
        let mut fiskeridir_vessel_ids = Vec::with_capacity(trips.len());

        for t in trips {
            let pg_range: PgRange<DateTime<Utc>> = PgRange {
                start: std::ops::Bound::Excluded(t.period.start()),
                end: std::ops::Bound::Included(t.period.end()),
            };
            range.push(pg_range);
            start_port_id.push(t.start_port_code);
            end_port_id.push(t.end_port_code);
            trip_assembler_ids.push(trip_assembler_id as i32);
            fiskeridir_vessel_ids.push(vessel_id.0);
        }

        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    trip_calculation_timers (fiskeridir_vessel_id, trip_assembler_id, timer)
VALUES
    ($1, $2, $3)
ON CONFLICT (fiskeridir_vessel_id, trip_assembler_id) DO
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
                range,
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

        sqlx::query!(
            r#"
INSERT INTO
    trips (
        period,
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
        $2::VARCHAR[],
        $3::VARCHAR[],
        $4::INT[],
        $5::BIGINT[]
    )
            "#,
            range,
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
}
