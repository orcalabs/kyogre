use fiskeridir_rs::{CallSign, SpeciesGroup};
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use kyogre_core::{
    AverageEeoiQuery, AverageFuiQuery, AverageTripBenchmarks, AverageTripBenchmarksQuery,
    DIESEL_LITER_CARBON_FACTOR, DateRange, EeoiQuery, EmptyVecToNone, EngineType,
    FiskeridirVesselId, FuiQuery, METERS_TO_NAUTICAL_MILES, MIN_EEOI_DISTANCE, Mmsi,
    ProcessingStatus, TripBenchmarksQuery, TripId, TripWithBenchmark,
};

use crate::{PostgresAdapter, error::Result, models::TripBenchmarkOutput};

impl PostgresAdapter {
    pub(crate) async fn add_benchmark_outputs(
        &self,
        values: &[kyogre_core::TripBenchmarkOutput],
    ) -> Result<()> {
        self.unnest_update_from::<_, _, TripBenchmarkOutput>(values, &self.pool)
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
    AVG(t.benchmark_fuel_consumption_liter) AS fuel_consumption_liter,
    AVG(t.benchmark_weight_per_hour) AS weight_per_hour,
    AVG(t.benchmark_weight_per_distance) AS weight_per_distance,
    AVG(t.benchmark_weight_per_fuel_liter) AS weight_per_fuel_liter,
    AVG(t.benchmark_catch_value_per_fuel_liter) AS catch_value_per_fuel_liter
FROM
    trips_detailed t
WHERE
    t.start_timestamp >= $1
    AND t.stop_timestamp <= $2
    AND (
        $3::INT IS NULL
        OR t.fiskeridir_length_group_id = $3
    )
    AND (
        $4::INT[] IS NULL
        OR t.haul_gear_group_ids && $4
    )
    AND (
        $5::BIGINT[] IS NULL
        OR t.fiskeridir_vessel_id = ANY ($5)
    )
            "#,
            query.range.start(),
            query.range.end(),
            query.length_group as Option<VesselLengthGroup>,
            query.gear_groups.as_slice().empty_to_none() as Option<&[GearGroup]>,
            query.vessel_ids.as_slice().empty_to_none() as Option<&[FiskeridirVesselId]>,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.into())
    }

    pub(crate) async fn trips_to_benchmark_impl(&self) -> Result<Vec<kyogre_core::BenchmarkTrip>> {
        Ok(sqlx::query_as!(
            kyogre_core::BenchmarkTrip,
            r#"
SELECT
    t.fiskeridir_vessel_id AS "vessel_id!: FiskeridirVesselId",
    trip_id AS "trip_id!: TripId",
    period AS "period: DateRange",
    period_precision AS "period_precision?: DateRange",
    CASE
        WHEN trip_assembler_id = 1 THEN landing_total_living_weight
        WHEN trip_assembler_id = 2 THEN haul_total_weight::DOUBLE PRECISION
        ELSE NULL
    END AS "total_catch_weight!",
    landing_total_price_for_fisher AS total_catch_value,
    distance,
    f.fiskeridir_length_group_id AS "vessel_length_group: VesselLengthGroup",
    f.engine_power_final AS engine_power,
    f.engine_building_year_final AS engine_building_year,
    f.auxiliary_engine_power,
    f.auxiliary_engine_building_year,
    f.boiler_engine_power,
    f.boiler_engine_building_year,
    f.engine_type_manual AS "engine_type: EngineType",
    f.engine_rpm_manual AS engine_rpm,
    f.service_speed,
    f.degree_of_electrification,
    w.call_sign AS "call_sign: CallSign",
    w.mmsi AS "mmsi: Mmsi"
FROM
    trips_detailed t
    INNER JOIN fiskeridir_vessels f ON t.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    INNER JOIN all_vessels w ON w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    benchmark_status = $1
            "#,
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(&self.pool)
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
            active_vessels
        WHERE
            call_sign = $1
    )
SELECT
    t.trip_id AS "id!: TripId",
    t.period AS "period!: DateRange",
    t.period_precision AS "period_precision: DateRange",
    t.benchmark_weight_per_hour AS weight_per_hour,
    t.benchmark_weight_per_distance AS weight_per_distance,
    t.benchmark_fuel_consumption_liter AS fuel_consumption_liter,
    t.benchmark_weight_per_fuel_liter AS weight_per_fuel_liter,
    t.benchmark_catch_value_per_fuel_liter AS catch_value_per_fuel_liter,
    t.benchmark_eeoi AS eeoi
FROM
    vessel_id v
    INNER JOIN trips_detailed t ON v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
WHERE
    (
        $2::TIMESTAMPTZ IS NULL
        OR LOWER(t.period) >= $2
    )
    AND (
        $3::TIMESTAMPTZ IS NULL
        OR UPPER(t.period) <= $3
    )
GROUP BY
    t.trip_id
ORDER BY
    CASE
        WHEN $4 = 1 THEN t.period
    END ASC,
    CASE
        WHEN $4 = 2 THEN t.period
    END DESC
            "#,
            query.call_sign.as_ref(),
            query.range.start(),
            query.range.end(),
            query.ordering as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(trips)
    }

    pub(crate) async fn fui_impl(&self, query: &FuiQuery) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"
WITH
    vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            active_vessels
        WHERE
            call_sign = $1
    )
SELECT
    CASE
        WHEN SUM(t.landing_total_living_weight) > 0
        AND SUM(t.distance) > $2 THEN (SUM(t.benchmark_fuel_consumption_liter) * $3)::DOUBLE PRECISION / (
            SUM(t.landing_total_living_weight)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
        )
        ELSE NULL
    END AS fui
FROM
    vessel_id v
    INNER JOIN trips_detailed t ON v.fiskeridir_vessel_id = t.fiskeridir_vessel_id
WHERE
    (
        $4::TIMESTAMPTZ IS NULL
        OR t.stop_timestamp >= $4
    )
    AND (
        $5::TIMESTAMPTZ IS NULL
        OR t.stop_timestamp <= $5
    )
            "#,
            query.call_sign.as_ref(),
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            query.range.start(),
            query.range.end(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.fui))
    }

    pub(crate) async fn eeoi_impl(&self, query: &EeoiQuery) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"
WITH
    vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            active_vessels
        WHERE
            call_sign = $1
    )
SELECT
    CASE
        WHEN SUM(t.landing_total_living_weight) > 0
        AND SUM(t.distance) > $2 THEN (SUM(t.benchmark_fuel_consumption_liter) * $3)::DOUBLE PRECISION / (
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
        OR t.stop_timestamp >= $5
    )
    AND (
        $6::TIMESTAMPTZ IS NULL
        OR t.stop_timestamp <= $6
    )
            "#,
            query.call_sign.as_ref(),
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            METERS_TO_NAUTICAL_MILES,
            query.range.start(),
            query.range.end(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.eeoi))
    }
    pub(crate) async fn average_fui_impl(&self, query: &AverageFuiQuery) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"

WITH
    fuis AS (
        SELECT
            CASE
                WHEN SUM(t.landing_total_living_weight) > 0
                AND SUM(t.distance) > $1 THEN (SUM(t.benchmark_fuel_consumption_liter) * $2)::DOUBLE PRECISION / (
                    SUM(t.landing_total_living_weight)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
                )
                ELSE NULL
            END AS fui
        FROM
            trips_detailed t
        WHERE
            t.stop_timestamp BETWEEN $3 AND $4
            AND (
                $5::INT IS NULL
                OR t.fiskeridir_length_group_id = $5
            )
            AND (
                $6::INT[] IS NULL
                OR t.haul_gear_group_ids && $6
            )
            AND (
                $7::BIGINT[] IS NULL
                OR t.fiskeridir_vessel_id = ANY ($7)
            )
            AND (
                $8::INT IS NULL
                OR t.landing_largest_quantum_species_group_id = $8
            )
        GROUP BY
            t.fiskeridir_vessel_id
    ),
    ranked_data AS (
        SELECT
            fui,
            percent_rank() OVER (
                ORDER BY
                    fui
            ) AS percent
        FROM
            fuis
    )
SELECT
    AVG(fui) AS fui
FROM
    ranked_data
WHERE
    (
        percent <= 0.95
        OR percent >= 0.05
    )
    OR (
        SELECT
            COUNT(*)
        FROM
            ranked_data
    ) <= 2
            "#,
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            query.range.start(),
            query.range.end(),
            query.length_group as Option<VesselLengthGroup>,
            query.gear_groups.as_slice().empty_to_none() as Option<&[GearGroup]>,
            query.vessel_ids.as_slice().empty_to_none() as Option<&[FiskeridirVesselId]>,
            query.species_group_id as Option<SpeciesGroup>
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.fui))
    }

    pub(crate) async fn average_eeoi_impl(&self, query: &AverageEeoiQuery) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"

WITH
    eeois AS (
        SELECT
            CASE
                WHEN SUM(t.landing_total_living_weight) > 0
                AND SUM(t.distance) > $1 THEN (SUM(t.benchmark_fuel_consumption_liter) * $2)::DOUBLE PRECISION / (
                    SUM(t.landing_total_living_weight * t.distance * $3)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
                )
                ELSE NULL
            END AS eeoi
        FROM
            trips_detailed t
        WHERE
            t.stop_timestamp BETWEEN $4 AND $5
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
            AND (
                $9::INT IS NULL
                OR t.landing_largest_quantum_species_group_id = $9
            )
        GROUP BY
            t.fiskeridir_vessel_id
    ),
    ranked_data AS (
        SELECT
            eeoi,
            percent_rank() OVER (
                ORDER BY
                    eeoi
            ) AS percent
        FROM
            eeois
    )
SELECT
    AVG(eeoi) AS eeoi
FROM
    ranked_data
WHERE
    (
        percent <= 0.95
        OR percent >= 0.05
    )
    OR (
        SELECT
            COUNT(*)
        FROM
            ranked_data
    ) <= 2
            "#,
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            METERS_TO_NAUTICAL_MILES,
            query.range.start(),
            query.range.end(),
            query.length_group as Option<VesselLengthGroup>,
            query.gear_groups.as_slice().empty_to_none() as Option<&[GearGroup]>,
            query.vessel_ids.as_slice().empty_to_none() as Option<&[FiskeridirVesselId]>,
            query.species_group_id as Option<SpeciesGroup>
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.eeoi))
    }
}
