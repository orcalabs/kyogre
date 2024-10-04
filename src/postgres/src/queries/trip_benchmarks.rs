use kyogre_core::{
    DateRange, FiskeridirVesselId, TripBenchmarkId, TripBenchmarksQuery, TripId,
    TripSustainabilityMetric, TripWithBenchmark, TripWithTotalLivingWeight,
};

use crate::{error::Result, models::TripBenchmarkOutput, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn add_benchmark_outputs(
        &self,
        values: Vec<kyogre_core::TripBenchmarkOutput>,
    ) -> Result<()> {
        self.unnest_insert_from::<_, _, TripBenchmarkOutput>(values, &self.pool)
            .await
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
    ) AS "weight_per_hour!"
FROM
    vessel_id v
    INNER JOIN trips t ON v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
    INNER JOIN trip_benchmark_outputs b ON t.trip_id = b.trip_id
WHERE
    NOT b.unrealistic
    AND (
        $3::TIMESTAMPTZ IS NULL
        OR LOWER(t.period) >= $3
    )
    AND (
        $4::TIMESTAMPTZ IS NULL
        OR UPPER(t.period) <= $4
    )
GROUP BY
    t.trip_id
HAVING
    ARRAY[$2] <@ ARRAY_AGG(b.trip_benchmark_id)
ORDER BY
    CASE
        WHEN $5 = 1 THEN t.period
    END ASC,
    CASE
        WHEN $5 = 2 THEN t.period
    END DESC
            "#,
            query.call_sign.as_ref(),
            TripBenchmarkId::WeightPerHour as i32,
            query.start_date,
            query.end_date,
            query.ordering as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn trips_with_landing_weight_impl(
        &self,
        id: FiskeridirVesselId,
    ) -> Result<Vec<TripWithTotalLivingWeight>> {
        let trips = sqlx::query_as!(
            TripWithTotalLivingWeight,
            r#"
SELECT
    t.trip_id AS "id!: TripId",
    t.period AS "period!: DateRange",
    t.period_precision AS "period_precision: DateRange",
    t.landing_total_living_weight AS "total_living_weight!"
FROM
    trips_detailed t
    LEFT JOIN trip_benchmark_outputs b ON t.trip_id = b.trip_id
    AND b.trip_benchmark_id = $1
WHERE
    t.fiskeridir_vessel_id = $2
    AND t.landing_total_living_weight > 0
    AND b.trip_id IS NULL
            "#,
            TripBenchmarkId::WeightPerHour as i32,
            id.into_inner(),
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
    ) AS "weight_per_hour!"
FROM
    trip_benchmark_outputs b
    INNER JOIN trips t ON b.trip_id = t.trip_id
WHERE
    t.fiskeridir_vessel_id = $2
    AND NOT b.unrealistic
GROUP BY
    b.trip_id
HAVING
    ARRAY[$1] <@ ARRAY_AGG(b.trip_benchmark_id)
            "#,
            TripBenchmarkId::WeightPerHour as i32,
            id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(metrics)
    }
}
