use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use futures::TryStreamExt;
use kyogre_core::{
    AverageEeoiQuery, AverageTripBenchmarks, AverageTripBenchmarksQuery, DateRange, EeoiQuery,
    EmptyVecToNone, FiskeridirVesselId, TripBenchmarkId, TripBenchmarkStatus, TripBenchmarksQuery,
    TripId, TripSustainabilityMetric, TripWithBenchmark, TripWithCatchValueAndFuel,
    TripWithDistance, TripWithTotalWeight, TripWithWeightAndFuel,
};

use crate::{error::Result, models::TripBenchmarkOutput, PostgresAdapter};

const METERS_TO_NAUTICAL_MILES: f64 = 1. / 1852.;

/// Fuel mass to CO2 mass conversion factor for Diesel/Gas Oil
/// Unit: CO2 (tonn) / Fuel (tonn)
///
/// Source: <https://www.classnk.or.jp/hp/pdf/activities/statutory/eedi/mepc_1-circ_684.pdf>
///         Appendix, section 3
const DIESEL_CARBON_FACTOR: f64 = 3.206;

const MIN_EEOI_DISTANCE: f64 = 1_000.;

impl PostgresAdapter {
    pub(crate) async fn add_benchmark_outputs(
        &self,
        values: &[kyogre_core::TripBenchmarkOutput],
    ) -> Result<()> {
        self.unnest_insert_from::<_, _, TripBenchmarkOutput>(values, &self.pool)
            .await
    }

    pub(crate) async fn average_trip_benchmarks_impl(
        &self,
        query: &AverageTripBenchmarksQuery,
    ) -> Result<AverageTripBenchmarks> {
        sqlx::query_as!(
            AverageTripBenchmarks,
            r#"
SELECT
    AVG(t.fuel_consumption) AS fuel_consumption,
    AVG(o.output) AS weight_per_hour,
    AVG(o2.output) AS weight_per_distance,
    AVG(o3.output) AS weight_per_fuel,
    AVG(o4.output) AS catch_value_per_fuel
FROM
    trips_detailed t
    LEFT JOIN trip_benchmark_outputs o ON t.trip_id = o.trip_id
    AND o.trip_benchmark_id = $1
    LEFT JOIN trip_benchmark_outputs o2 ON t.trip_id = o2.trip_id
    AND o2.trip_benchmark_id = $2
    LEFT JOIN trip_benchmark_outputs o3 ON t.trip_id = o3.trip_id
    AND o3.trip_benchmark_id = $3
    LEFT JOIN trip_benchmark_outputs o4 ON t.trip_id = o4.trip_id
    AND o4.trip_benchmark_id = $4
WHERE
    t.start_timestamp >= $5
    AND t.stop_timestamp <= $6
    AND (
        $7::INT IS NULL
        OR t.fiskeridir_length_group_id = $7
    )
    AND (
        $8::INT[] IS NULL
        OR t.haul_gear_group_ids && $8
    )
    AND (
        $9::BIGINT[] IS NULL
        OR t.fiskeridir_vessel_id = ANY ($9)
    )
            "#,
            TripBenchmarkId::WeightPerHour as i32,
            TripBenchmarkId::WeightPerDistance as i32,
            TripBenchmarkId::WeightPerFuel as i32,
            TripBenchmarkId::CatchValuePerFuel as i32,
            query.start_date,
            query.end_date,
            query.length_group as Option<VesselLengthGroup>,
            query.gear_groups.as_slice().empty_to_none() as Option<&[GearGroup]>,
            query.vessel_ids.as_slice().empty_to_none() as Option<&[FiskeridirVesselId]>,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.into())
    }

    pub(crate) async fn trip_benchmarks_impl(
        &self,
        query: &TripBenchmarksQuery,
    ) -> Result<Vec<TripWithBenchmark>> {
        let trips = sqlx::query_as!(
            TripWithBenchmark,
            r#"
WITH
    vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            fiskeridir_ais_vessel_mapping_whitelist
        WHERE
            call_sign = $1
    )
SELECT
    t.trip_id AS "id!: TripId",
    t.period AS "period!: DateRange",
    t.period_precision AS "period_precision: DateRange",
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $2
    ) AS weight_per_hour,
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $3
    ) AS weight_per_distance,
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $4
    ) AS fuel_consumption,
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $5
    ) AS weight_per_fuel,
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $6
    ) AS catch_value_per_fuel
FROM
    vessel_id v
    INNER JOIN trips t ON v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    LEFT JOIN trip_benchmark_outputs b ON t.trip_id = b.trip_id
WHERE
    (
        b.unrealistic IS NULL
        OR NOT b.unrealistic
    )
    AND (
        $7::TIMESTAMPTZ IS NULL
        OR LOWER(t.period) >= $7
    )
    AND (
        $8::TIMESTAMPTZ IS NULL
        OR UPPER(t.period) <= $8
    )
GROUP BY
    t.trip_id
ORDER BY
    CASE
        WHEN $9 = 1 THEN t.period
    END ASC,
    CASE
        WHEN $9 = 2 THEN t.period
    END DESC
            "#,
            query.call_sign.as_ref(),
            TripBenchmarkId::WeightPerHour as i32,
            TripBenchmarkId::WeightPerDistance as i32,
            TripBenchmarkId::FuelConsumption as i32,
            TripBenchmarkId::WeightPerFuel as i32,
            TripBenchmarkId::CatchValuePerFuel as i32,
            query.start_date,
            query.end_date,
            query.ordering as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn eeoi_impl(&self, query: &EeoiQuery) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"
WITH
    vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            fiskeridir_ais_vessel_mapping_whitelist
        WHERE
            call_sign = $1
    )
SELECT
    CASE
        WHEN SUM(t.landing_total_living_weight) > 0
        AND SUM(t.distance) > $2 THEN (SUM(t.fuel_consumption) * $3)::DOUBLE PRECISION / (
            SUM(t.landing_total_living_weight * t.distance * $4)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
        )
        ELSE NULL
    END AS eeoi
FROM
    vessel_id v
    INNER JOIN trips_detailed t ON v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
WHERE
    (
        $5::TIMESTAMPTZ IS NULL
        OR t.start_timestamp >= $5
    )
    AND (
        $6::TIMESTAMPTZ IS NULL
        OR t.stop_timestamp <= $6
    )
            "#,
            query.call_sign.as_ref(),
            MIN_EEOI_DISTANCE,
            DIESEL_CARBON_FACTOR,
            METERS_TO_NAUTICAL_MILES,
            query.start_date,
            query.end_date,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.eeoi))
    }

    pub(crate) async fn average_eeoi_impl(&self, query: &AverageEeoiQuery) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"

WITH
    eeois AS (
        SELECT
            CASE
                WHEN SUM(t.landing_total_living_weight) > 0
                AND SUM(t.distance) > $1 THEN (SUM(t.fuel_consumption) * $2)::DOUBLE PRECISION / (
                    SUM(t.landing_total_living_weight * t.distance * $3)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
                )
                ELSE NULL
            END AS eeoi
        FROM
            trips_detailed t
        WHERE
            t.start_timestamp >= $4
            AND t.stop_timestamp <= $5
            AND (
                $6::INT IS NULL
                OR t.fiskeridir_length_group_id = $6
            )
            AND (
                $7::INT[] IS NULL
                OR t.haul_gear_group_ids && $7
            )
            AND (
                $8::BIGINT[] IS NULL
                OR t.fiskeridir_vessel_id = ANY ($8)
            )
        GROUP BY
            t.fiskeridir_vessel_id
    )
SELECT
    AVG(eeoi) AS eeoi
FROM
    eeois
            "#,
            MIN_EEOI_DISTANCE,
            DIESEL_CARBON_FACTOR,
            METERS_TO_NAUTICAL_MILES,
            query.start_date,
            query.end_date,
            query.length_group as Option<VesselLengthGroup>,
            query.gear_groups.as_slice().empty_to_none() as Option<&[GearGroup]>,
            query.vessel_ids.as_slice().empty_to_none() as Option<&[FiskeridirVesselId]>,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.eeoi))
    }

    pub(crate) async fn trips_without_fuel_consumption_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripId>> {
        let ids = sqlx::query!(
            r#"
SELECT
    t.trip_id AS "id!: TripId"
FROM
    trips t
    LEFT JOIN trip_benchmark_outputs b ON t.trip_id = b.trip_id
    AND b.trip_benchmark_id = $1
WHERE
    t.fiskeridir_vessel_id = $2
    AND (
        b.trip_id IS NULL
        OR b.status = $3
    )
            "#,
            TripBenchmarkId::FuelConsumption as i32,
            id.into_inner(),
            TripBenchmarkStatus::MustRecompute as i32,
        )
        .fetch(&self.pool)
        .map_ok(|v| v.id)
        .try_collect()
        .await?;

        Ok(ids)
    }

    pub(crate) async fn trips_with_weight_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripWithTotalWeight>> {
        let trips = sqlx::query_as!(
            TripWithTotalWeight,
            r#"
SELECT
    t.trip_id AS "id!: TripId",
    t.period AS "period!: DateRange",
    t.period_precision AS "period_precision: DateRange",
    CASE
        WHEN t.trip_assembler_id = 1 THEN t.landing_total_living_weight
        ELSE t.haul_total_weight
    END AS "total_weight!"
FROM
    trips_detailed t
    LEFT JOIN trip_benchmark_outputs b ON t.trip_id = b.trip_id
    AND b.trip_benchmark_id = $1
WHERE
    t.fiskeridir_vessel_id = $2
    AND CASE
        WHEN t.trip_assembler_id = 1 THEN t.landing_total_living_weight
        ELSE t.haul_total_weight
    END > 0
    AND (
        b.trip_id IS NULL
        OR b.status = $3
    )
            "#,
            TripBenchmarkId::WeightPerHour as i32,
            id.into_inner(),
            TripBenchmarkStatus::MustRecompute as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn trips_with_distance_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripWithDistance>> {
        let trips = sqlx::query_as!(
            TripWithDistance,
            r#"
SELECT
    t.trip_id AS "id!: TripId",
    t.distance AS "distance!",
    CASE
        WHEN t.trip_assembler_id = 1 THEN t.landing_total_living_weight
        ELSE t.haul_total_weight
    END AS "total_weight!"
FROM
    trips_detailed t
    LEFT JOIN trip_benchmark_outputs b ON t.trip_id = b.trip_id
    AND b.trip_benchmark_id = $1
WHERE
    t.fiskeridir_vessel_id = $2
    AND CASE
        WHEN t.trip_assembler_id = 1 THEN t.landing_total_living_weight
        ELSE t.haul_total_weight
    END > 0
    AND t.distance > 0
    AND (
        b.trip_id IS NULL
        OR b.status = $3
    )
            "#,
            TripBenchmarkId::WeightPerDistance as i32,
            id.into_inner(),
            TripBenchmarkStatus::MustRecompute as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn trips_with_weight_and_fuel_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripWithWeightAndFuel>> {
        let trips = sqlx::query_as!(
            TripWithWeightAndFuel,
            r#"
SELECT
    t.trip_id AS "id!: TripId",
    CASE
        WHEN t.trip_assembler_id = 1 THEN t.landing_total_living_weight
        ELSE t.haul_total_weight
    END AS "total_weight!",
    b_fuel.output AS "fuel_consumption!"
FROM
    trips_detailed t
    LEFT JOIN trip_benchmark_outputs b_fuel ON t.trip_id = b_fuel.trip_id
    AND b_fuel.trip_benchmark_id = $1
    LEFT JOIN trip_benchmark_outputs b_weight ON t.trip_id = b_weight.trip_id
    AND b_weight.trip_benchmark_id = $2
WHERE
    t.fiskeridir_vessel_id = $3
    AND CASE
        WHEN t.trip_assembler_id = 1 THEN t.landing_total_living_weight
        ELSE t.haul_total_weight
    END > 0
    AND b_fuel.output > 0
    AND (
        b_weight.trip_id IS NULL
        OR b_weight.status = $4
    )
            "#,
            TripBenchmarkId::FuelConsumption as i32,
            TripBenchmarkId::WeightPerFuel as i32,
            id.into_inner(),
            TripBenchmarkStatus::MustRecompute as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn trips_with_catch_value_and_fuel_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripWithCatchValueAndFuel>> {
        let trips = sqlx::query_as!(
            TripWithCatchValueAndFuel,
            r#"
SELECT
    t.trip_id AS "id!: TripId",
    SUM(l.price_for_fisher)::DOUBLE PRECISION AS "total_catch_value!: f64",
    MAX(b_fuel.output) AS "fuel_consumption!"
FROM
    trips_detailed t
    INNER JOIN landing_entries l ON l.landing_id = ANY (t.landing_ids)
    LEFT JOIN trip_benchmark_outputs b_fuel ON t.trip_id = b_fuel.trip_id
    AND b_fuel.trip_benchmark_id = $1
    LEFT JOIN trip_benchmark_outputs b_cv ON t.trip_id = b_cv.trip_id
    AND b_cv.trip_benchmark_id = $2
WHERE
    t.fiskeridir_vessel_id = $3
    AND l.price_for_fisher IS NOT NULL
    AND b_fuel.output > 0
    AND (
        b_cv.trip_id IS NULL
        OR b_cv.status = $4
    )
GROUP BY
    t.trip_id
            "#,
            TripBenchmarkId::FuelConsumption as i32,
            TripBenchmarkId::CatchValuePerFuel as i32,
            id.into_inner(),
            TripBenchmarkStatus::MustRecompute as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn sustainability_metrics_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripSustainabilityMetric>> {
        let metrics = sqlx::query_as!(
            TripSustainabilityMetric,
            r#"
SELECT
    b.trip_id AS "id!: TripId",
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $1
    ) AS "weight_per_hour!",
    MAX(b.output) FILTER (
        WHERE
            b.trip_benchmark_id = $2
    ) AS "weight_per_distance!"
FROM
    trip_benchmark_outputs b
    INNER JOIN trips t ON b.trip_id = t.trip_id
WHERE
    t.fiskeridir_vessel_id = $3
    AND NOT b.unrealistic
GROUP BY
    b.trip_id
HAVING
    ARRAY[$1] <@ ARRAY_AGG(b.trip_benchmark_id)
            "#,
            TripBenchmarkId::WeightPerHour as i32,
            TripBenchmarkId::WeightPerDistance as i32,
            id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }

    pub(crate) async fn reset_trip_benchmarks(
        &self,
        id: TripId,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trip_benchmark_outputs
SET
    status = $1
WHERE
    trip_id = $2
            "#,
            TripBenchmarkStatus::MustRecompute as i32,
            id.into_inner(),
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
