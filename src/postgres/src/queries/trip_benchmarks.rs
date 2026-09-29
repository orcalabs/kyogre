use crate::models::AverageVesselsBenchmarks;
use crate::{PostgresAdapter, error::Result, models::TripBenchmarkOutput};
use fiskeridir_rs::{CallSign, SpeciesGroup};
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use kyogre_core::{
    AverageEeoiQuery, AverageFuiQuery, AverageTripBenchmarks, AverageTripBenchmarksQuery,
    BarentswatchUserId, DIESEL_LITER_CARBON_FACTOR, DateRange, EeoiQuery, EmptyVecToNone,
    EngineType, FiskeridirVesselId, FuiQuery, METERS_TO_NAUTICAL_MILES, MIN_EEOI_DISTANCE, Mmsi,
    PerVesselBenchmarkParams, ProcessingStatus, SumVesselBenchmark, TripBenchmarksQuery, TripId,
    TripWithBenchmark,
};

impl PostgresAdapter {
    pub(crate) async fn similar_or_following_vessels(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
        query: &PerVesselBenchmarkParams,
    ) -> Result<Vec<FiskeridirVesselId>> {
        let logged_in_vessel = sqlx::query!(
            r#"
SELECT
    fiskeridir_vessel_id AS "id: FiskeridirVesselId"
FROM
    all_vessels
WHERE
    call_sign = $1
            "#,
            call_sign.as_ref()
        )
        .fetch_optional(&self.pool)
        .await?;

        let logged_in_vessel = if let Some(logged_in_vessel) = logged_in_vessel.map(|r| r.id) {
            logged_in_vessel
        } else {
            return Ok(vec![]);
        };

        let mut vessels = if query.use_following_list.unwrap_or(false) {
            sqlx::query!(
                r#"
SELECT
    COALESCE(ARRAY_AGG(fiskeridir_vessel_id), '{}') AS "ids!: Vec<FiskeridirVesselId>"
FROM
    user_follows
WHERE
    barentswatch_user_id = $1
                 "#,
                user_id.as_ref()
            )
            .fetch_one(&self.pool)
            .await?
            .ids
        } else {
            let permissions = sqlx::query!(
                r#"
SELECT
    COALESCE(ARRAY_AGG(permission_type), '{}') AS "permissions!"
FROM
    vessel_permissions p
WHERE
    fiskeridir_vessel_id = $1
                "#,
                logged_in_vessel as FiskeridirVesselId
            )
            .fetch_one(&self.pool)
            .await?
            .permissions;

            if permissions.is_empty() {
                sqlx::query!(
                    r#"
SELECT
    COALESCE(ARRAY_AGG(f2.fiskeridir_vessel_id), '{}') AS "ids!: Vec<FiskeridirVesselId>"
FROM
    fiskeridir_vessels f
    INNER JOIN fiskeridir_vessels f2 ON f.gear_group_ids && f2.gear_group_ids
    AND f.fiskeridir_length_group_id = f2.fiskeridir_length_group_id
    AND f2.fiskeridir_vessel_id != $1
WHERE
    f.fiskeridir_vessel_id = $1
                "#,
                    logged_in_vessel as FiskeridirVesselId
                )
                .fetch_one(&self.pool)
                .await?
                .ids
            } else {
                sqlx::query!(
                    r#"
SELECT
    COALESCE(
        ARRAY_AGG(
            CASE
                WHEN vessel_one = $1 THEN vessel_two
                WHEN vessel_two = $1 THEN vessel_one
            END
        ),
        '{}'
    ) AS "ids!: Vec<FiskeridirVesselId>"
FROM
    vessel_catch_similarity_distances v
WHERE
    (
        vessel_one = $1
        OR vessel_two = $1
    )
    AND distance < 0.4
                "#,
                    logged_in_vessel as FiskeridirVesselId,
                )
                .fetch_one(&self.pool)
                .await?
                .ids
            }
        };

        vessels.push(logged_in_vessel);

        Ok(vessels)
    }
    pub(crate) async fn per_vessel_benchmarks_sum_impl(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
        query: &PerVesselBenchmarkParams,
    ) -> Result<Vec<SumVesselBenchmark>> {
        let vessels = self
            .similar_or_following_vessels(user_id, call_sign, query)
            .await?;
        let out = sqlx::query_as!(
            SumVesselBenchmark,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    SUM(benchmark_weight_per_hour) AS sum_weight_per_hour,
    SUM(benchmark_weight_per_distance) AS sum_weight_per_distance,
    SUM(landing_total_living_weight) AS sum_living_weight
FROM
    trips_detailed
WHERE
    start_timestamp >= $1
    AND stop_timestamp <= $2
    AND fiskeridir_vessel_id = ANY ($3)
GROUP BY
    fiskeridir_vessel_id
            "#,
            query.range.start(),
            query.range.end(),
            &vessels as &[FiskeridirVesselId]
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(out)
    }
    pub(crate) async fn per_vessel_benchmarks_avg_impl(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
        query: &PerVesselBenchmarkParams,
    ) -> Result<Option<kyogre_core::AverageVesselsBenchmarks>> {
        let vessels = self
            .similar_or_following_vessels(user_id, call_sign, query)
            .await?;

        let own_eeoi = self
            .eeoi_impl(&EeoiQuery {
                call_sign: call_sign.clone(),
                range: query.range.clone().into(),
            })
            .await?;

        let own_fui = self
            .fui_impl(&FuiQuery {
                call_sign: call_sign.clone(),
                range: query.range.clone().into(),
            })
            .await?;

        let average_eeoi = self
            .average_eeoi_impl(&AverageEeoiQuery {
                range: query.range.clone().into(),
                gear_groups: vec![],
                length_group: None,
                vessel_ids: vessels.clone(),
                species_group_id: None,
            })
            .await?;

        let average_fui = self
            .average_fui_impl(&AverageFuiQuery {
                range: query.range.clone().into(),
                gear_groups: vec![],
                length_group: None,
                vessel_ids: vessels.clone(),
                species_group_id: None,
            })
            .await?;

        let out = sqlx::query_as!(
            AverageVesselsBenchmarks,
            r#"
WITH
    logged_in_vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            all_vessels
        WHERE
            call_sign = $3
    ),
    logged_in_user_values AS (
        SELECT
            AVG(benchmark_fuel_consumption_liter) AS own_average_fuel_consumption_liter,
            AVG(benchmark_weight_per_hour) AS own_average_weight_per_hour,
            AVG(benchmark_weight_per_distance) AS own_average_weight_per_distance,
            AVG(benchmark_weight_per_fuel_liter) AS own_average_weight_per_fuel_liter,
            AVG(benchmark_catch_value_per_fuel_liter) AS own_average_catch_value_per_fuel_liter,
            AVG(landing_total_living_weight) AS own_average_living_weight
        FROM
            trips_detailed
        WHERE
            start_timestamp >= $1
            AND stop_timestamp <= $2
            AND fiskeridir_vessel_id = (
                SELECT
                    fiskeridir_vessel_id
                FROM
                    logged_in_vessel_id
            )
    ),
    other_vessels AS (
        SELECT
            MAX(average_weight_per_hour) AS highest_average_weight_per_hour,
            MAX(average_fuel_consumption_liter) AS highest_average_fuel_consumption_liter,
            MAX(average_weight_per_distance) AS highest_average_weight_per_distance,
            MAX(average_weight_per_fuel_liter) AS highest_average_weight_per_fuel_liter,
            MAX(average_catch_value_per_fuel_liter) AS highest_average_catch_value_per_fuel_liter,
            MAX(average_living_weight) AS highest_average_living_weight
        FROM
            (
                SELECT
                    AVG(
                        CASE
                            WHEN fiskeridir_vessel_id = (
                                SELECT
                                    fiskeridir_vessel_id
                                FROM
                                    logged_in_vessel_id
                            ) THEN benchmark_fuel_consumption_liter
                            ELSE benchmark_fuel_consumption_liter_estimated_only
                        END
                    ) AS average_fuel_consumption_liter,
                    AVG(benchmark_weight_per_hour) AS average_weight_per_hour,
                    AVG(benchmark_weight_per_distance) AS average_weight_per_distance,
                    AVG(
                        CASE
                            WHEN fiskeridir_vessel_id = (
                                SELECT
                                    fiskeridir_vessel_id
                                FROM
                                    logged_in_vessel_id
                            ) THEN benchmark_weight_per_fuel_liter
                            ELSE benchmark_weight_per_fuel_liter_estimated_only
                        END
                    ) AS average_weight_per_fuel_liter,
                    AVG(
                        CASE
                            WHEN fiskeridir_vessel_id = (
                                SELECT
                                    fiskeridir_vessel_id
                                FROM
                                    logged_in_vessel_id
                            ) THEN benchmark_catch_value_per_fuel_liter
                            ELSE benchmark_catch_value_per_fuel_liter_estimated_only
                        END
                    ) AS average_catch_value_per_fuel_liter,
                    AVG(landing_total_living_weight) AS average_living_weight
                FROM
                    trips_detailed
                WHERE
                    start_timestamp >= $1
                    AND stop_timestamp <= $2
                    AND fiskeridir_vessel_id = ANY ($4)
                GROUP BY
                    fiskeridir_vessel_id
            ) q
    )
SELECT
    *
FROM
    other_vessels o
    INNER JOIN logged_in_user_values l ON TRUE
            "#,
            query.range.start(),
            query.range.end(),
            call_sign.as_ref(),
            &vessels as &[FiskeridirVesselId]
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(out.map(|a| kyogre_core::AverageVesselsBenchmarks {
            highest_average_fuel_consumption_liter: a.highest_average_fuel_consumption_liter,
            highest_average_weight_per_hour: a.highest_average_weight_per_hour,
            highest_average_weight_per_distance: a.highest_average_weight_per_distance,
            highest_average_weight_per_fuel_liter: a.highest_average_weight_per_fuel_liter,
            highest_average_catch_value_per_fuel_liter: a
                .highest_average_catch_value_per_fuel_liter,
            highest_average_living_weight: a.highest_average_living_weight,
            average_eeoi,
            average_fui,
            own_average_fuel_consumption_liter: a.own_average_fuel_consumption_liter,
            own_average_weight_per_hour: a.own_average_weight_per_hour,
            own_average_weight_per_distance: a.own_average_weight_per_distance,
            own_average_weight_per_fuel_liter: a.own_average_weight_per_fuel_liter,
            own_average_catch_value_per_fuel_liter: a.own_average_catch_value_per_fuel_liter,
            own_average_living_weight: a.own_average_living_weight,
            own_eeoi,
            own_fui,
        }))
    }
    pub(crate) async fn add_benchmark_output(
        &self,
        value: &kyogre_core::TripBenchmarkOutput,
    ) -> Result<()> {
        self.unnest_update_from::<_, _, TripBenchmarkOutput>(
            std::slice::from_ref(value),
            &self.pool,
        )
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
    td.fiskeridir_vessel_id AS "vessel_id!: FiskeridirVesselId",
    td.trip_id AS "trip_id!: TripId",
    td.period AS "period: DateRange",
    td.period_precision AS "period_precision?: DateRange",
    CASE
        WHEN td.trip_assembler_id = 1 THEN td.landing_total_living_weight
        WHEN td.trip_assembler_id = 2 THEN td.haul_total_weight::DOUBLE PRECISION
        ELSE NULL
    END AS "total_catch_weight!",
    td.landing_total_price_for_fisher AS total_catch_value,
    td.distance,
    td.benchmark_state_counter,
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
    trips_detailed td
    INNER JOIN fiskeridir_vessels f ON td.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    INNER JOIN all_vessels w ON w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    td.benchmark_status = $1
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
    percent BETWEEN 0.05 AND 0.95
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
    percent BETWEEN 0.05 AND 0.95
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
