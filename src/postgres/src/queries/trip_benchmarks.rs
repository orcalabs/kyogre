use chrono::{DateTime, Utc};
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use futures::TryStreamExt;
use kyogre_core::{
    DateRange, FiskeridirVesselId, TripBenchmarkId, TripBenchmarkStatus, TripBenchmarksQuery,
    TripId, TripSustainabilityMetric, TripWithBenchmark, TripWithDistance, TripWithTotalWeight,
    TripWithWeightAndFuel,
};

use crate::{error::Result, models::TripBenchmarkOutput, PostgresAdapter};

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
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        gear_groups: Vec<GearGroup>,
        length_group: Option<VesselLengthGroup>,
    ) -> Result<kyogre_core::AverageTripBenchmarks> {
        let gear_groups = (!gear_groups.is_empty()).then_some(gear_groups);
        Ok(sqlx::query_as!(
            kyogre_core::AverageTripBenchmarks,
            r#"
SELECT
    AVG(t.fuel_consumption) AS fuel_consumption,
    AVG(o.output) AS weight_per_hour,
    AVG(o2.output) AS weight_per_distance,
    AVG(o3.output) AS weight_per_fuel
FROM
    trips_detailed t
    LEFT JOIN trip_benchmark_outputs o ON t.trip_id = o.trip_id
    AND o.trip_benchmark_id = $1
    LEFT JOIN trip_benchmark_outputs o2 ON t.trip_id = o2.trip_id
    AND o2.trip_benchmark_id = $2
    LEFT JOIN trip_benchmark_outputs o3 ON t.trip_id = o3.trip_id
    AND o3.trip_benchmark_id = $3
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
            "#,
            TripBenchmarkId::WeightPerHour as i32,
            TripBenchmarkId::WeightPerDistance as i32,
            TripBenchmarkId::WeightPerFuel as i32,
            start_date,
            end_date,
            &length_group as &Option<VesselLengthGroup>,
            &gear_groups as &Option<Vec<GearGroup>>
        )
        .fetch_one(&self.pool)
        .await?)
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
    ) AS weight_per_fuel
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
        $6::TIMESTAMPTZ IS NULL
        OR LOWER(t.period) >= $6
    )
    AND (
        $7::TIMESTAMPTZ IS NULL
        OR UPPER(t.period) <= $7
    )
GROUP BY
    t.trip_id
ORDER BY
    CASE
        WHEN $8 = 1 THEN t.period
    END ASC,
    CASE
        WHEN $8 = 2 THEN t.period
    END DESC
            "#,
            query.call_sign.as_ref(),
            TripBenchmarkId::WeightPerHour as i32,
            TripBenchmarkId::WeightPerDistance as i32,
            TripBenchmarkId::FuelConsumption as i32,
            TripBenchmarkId::WeightPerFuel as i32,
            query.start_date,
            query.end_date,
            query.ordering as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
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
    AND b_weight.trip_id IS NULL
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
}
