use crate::models::AverageVesselsBenchmarks;
use crate::{PostgresAdapter, error::Result, models::TripBenchmarkOutput};
use fiskeridir_rs::{CallSign, SpeciesGroup};
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use kyogre_core::{
    AverageCarbonIntensityQuery, AverageEeoiQuery, AverageTripBenchmarks,
    AverageTripBenchmarksQuery, BarentswatchUserId, CarbonIntensityQuery,
    DIESEL_LITER_CARBON_FACTOR, DateRange, EeoiQuery, EmptyVecToNone, EngineType,
    FiskeridirVesselId, METERS_TO_NAUTICAL_MILES, MIN_EEOI_DISTANCE, Mmsi,
    PerVesselBenchmarkParams, PerVesselStats, ProcessingStatus, TripBenchmarksQuery, TripId,
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
                "#,
                logged_in_vessel as FiskeridirVesselId,
            )
            .fetch_one(&self.pool)
            .await?
            .ids
        };

        vessels.push(logged_in_vessel);

        Ok(vessels)
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
                species_group_ids: query.species_group_ids.clone(),
            })
            .await?;

        let own_carbon_intensity = self
            .carbon_intensity_impl(&CarbonIntensityQuery {
                call_sign: call_sign.clone(),
                range: query.range.clone().into(),
                species_group_ids: query.species_group_ids.clone(),
            })
            .await?;

        let average_eeoi = self
            .average_eeoi_impl(
                Some(call_sign),
                &AverageEeoiQuery {
                    range: query.range.clone().into(),
                    gear_groups: vec![],
                    length_group: None,
                    vessel_ids: vessels.clone(),
                    species_group_id: None,
                    species_group_ids: query.species_group_ids.clone(),
                },
            )
            .await?;

        let average_carbon_intensity = self
            .average_carbon_intensity_impl(
                Some(call_sign),
                &AverageCarbonIntensityQuery {
                    range: query.range.clone().into(),
                    gear_groups: vec![],
                    length_group: None,
                    vessel_ids: vessels.clone(),
                    species_group_id: None,
                    species_group_ids: query.species_group_ids.clone(),
                },
            )
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
            SUM(benchmark_fuel_consumption_liter) AS own_sum_fuel_consumption_liter,
            SUM(landing_total_living_weight) / NULLIF(
                (
                    SUM(
                        EXTRACT(
                            EPOCH
                            FROM
                                UPPER(COALESCE(period_precision, period)) - LOWER(COALESCE(period_precision, period))
                        )
                    ) / (60 * 60)
                ),
                0.0
            ) AS own_sum_weight_per_hour,
            SUM(landing_total_living_weight) / NULLIF(SUM((distance * $6)), 0.0) AS own_sum_weight_per_distance,
            SUM(landing_total_living_weight) / NULLIF(SUM(benchmark_fuel_consumption_liter), 0.0) AS own_sum_weight_per_fuel_liter,
            SUM(landing_total_price_for_fisher) / NULLIF(SUM(benchmark_fuel_consumption_liter), 0.0) AS own_sum_catch_value_per_fuel_liter,
            SUM(landing_total_living_weight) AS own_sum_living_weight,
            COUNT(*)::BIGINT AS own_num_trips,
            SUM(CARDINALITY(haul_ids))::BIGINT AS own_num_hauls,
            SUM(landing_total_price_for_fisher) AS own_value_nok,
            BOOL_OR(price_for_fisher_is_estimated) AS own_trips_has_price_estimation,
            SUM(
                benchmark_percentage_of_trip_covered_by_measurements
            ) / NULLIF(COUNT(*), 0) AS own_fuel_measurement_coverage_percentage,
            SUM(
                EXTRACT(
                    EPOCH
                    FROM
                        UPPER(COALESCE(period_precision, period)) - LOWER(COALESCE(period_precision, period))
                )
            )::BIGINT AS own_trip_duration_seconds
        FROM
            trips_detailed
        WHERE
            stop_timestamp >= $1
            AND stop_timestamp <= $2
            AND fiskeridir_vessel_id = (
                SELECT
                    fiskeridir_vessel_id
                FROM
                    logged_in_vessel_id
            )
            AND (
                $4::INT[] IS NULL
                OR landing_two_highest_species_group_quantum && $4
            )
    ),
    other_vessel_trips AS (
        SELECT
            fiskeridir_vessel_id,
            SUM(
                CASE
                    WHEN fiskeridir_vessel_id = (
                        SELECT
                            fiskeridir_vessel_id
                        FROM
                            logged_in_vessel_id
                    ) THEN benchmark_fuel_consumption_liter
                    ELSE benchmark_fuel_consumption_liter_estimated_only
                END
            ) AS sum_fuel_consumption_liter,
            SUM(landing_total_living_weight) / NULLIF(
                (
                    SUM(
                        EXTRACT(
                            EPOCH
                            FROM
                                UPPER(COALESCE(period_precision, period)) - LOWER(COALESCE(period_precision, period))
                        )
                    ) / (60 * 60)
                ),
                0.0
            ) AS sum_weight_per_hour,
            SUM(landing_total_living_weight) / NULLIF(SUM((distance * $6)), 0.0) AS sum_weight_per_distance,
            SUM(landing_total_living_weight) / NULLIF(
                SUM(
                    CASE
                        WHEN fiskeridir_vessel_id = (
                            SELECT
                                fiskeridir_vessel_id
                            FROM
                                logged_in_vessel_id
                        ) THEN benchmark_fuel_consumption_liter
                        ELSE benchmark_fuel_consumption_liter_estimated_only
                    END
                ),
                0.0
            ) AS sum_weight_per_fuel_liter,
            SUM(landing_total_price_for_fisher) / NULLIF(
                SUM(
                    CASE
                        WHEN fiskeridir_vessel_id = (
                            SELECT
                                fiskeridir_vessel_id
                            FROM
                                logged_in_vessel_id
                        ) THEN benchmark_fuel_consumption_liter
                        ELSE benchmark_fuel_consumption_liter_estimated_only
                    END
                ),
                0.0
            ) AS sum_catch_value_per_fuel_liter,
            SUM(landing_total_living_weight) AS sum_living_weight,
            SUM(landing_total_price_for_fisher) AS value_nok,
            COUNT(*) AS num_trips,
            SUM(CARDINALITY(haul_ids)) AS num_hauls,
            SUM(
                EXTRACT(
                    EPOCH
                    FROM
                        UPPER(COALESCE(period_precision, period)) - LOWER(COALESCE(period_precision, period))
                )::BIGINT
            ) AS trip_duration_seconds
        FROM
            trips_detailed
        WHERE
            stop_timestamp >= $1
            AND stop_timestamp <= $2
            AND (
                $4::INT[] IS NULL
                OR landing_two_highest_species_group_quantum && $4
            )
            AND fiskeridir_vessel_id = ANY ($5)
        GROUP BY
            fiskeridir_vessel_id
    ),
    other_vessels AS (
        SELECT
            MAX(sum_weight_per_hour) AS highest_weight_per_hour,
            MIN(sum_fuel_consumption_liter) AS lowest_fuel_consumption_liter,
            MAX(sum_weight_per_distance) AS highest_weight_per_distance,
            MAX(sum_weight_per_fuel_liter) AS highest_weight_per_fuel_liter,
            MAX(sum_catch_value_per_fuel_liter) AS highest_catch_value_per_fuel_liter,
            MAX(sum_living_weight) AS highest_living_weight,
            AVG(sum_weight_per_hour) AS average_weight_per_hour,
            AVG(sum_fuel_consumption_liter) AS average_fuel_consumption_liter,
            AVG(sum_weight_per_distance) AS average_weight_per_distance,
            AVG(sum_weight_per_fuel_liter) AS average_weight_per_fuel_liter,
            AVG(sum_catch_value_per_fuel_liter) AS average_catch_value_per_fuel_liter,
            AVG(sum_living_weight) AS average_living_weight,
            AVG(num_trips)::BIGINT AS average_num_trips,
            AVG(num_hauls)::BIGINT AS average_num_hauls,
            AVG(trip_duration_seconds)::BIGINT AS average_trip_duration_seconds,
            AVG(value_nok) AS average_value_nok
        FROM
            other_vessel_trips
    ),
    other_vessel_stats AS (
        SELECT
            COALESCE(
                JSONB_AGG(
                    JSONB_BUILD_OBJECT(
                        'fiskeridirVesselId',
                        fiskeridir_vessel_id,
                        'weightPerHour',
                        sum_weight_per_hour,
                        'fuelConsumptionLiter',
                        sum_fuel_consumption_liter,
                        'weightPerDistance',
                        sum_weight_per_distance,
                        'weightPerFuelLiter',
                        sum_weight_per_fuel_liter,
                        'catchValuePerFuelLiter',
                        sum_catch_value_per_fuel_liter,
                        'livingWeight',
                        sum_living_weight,
                        'numTrips',
                        num_trips::BIGINT,
                        'numHauls',
                        num_hauls::BIGINT,
                        'tripDurationSeconds',
                        trip_duration_seconds::BIGINT,
                        'valueNok',
                        value_nok
                    )
                ),
                '[]'
            )::TEXT AS "per_vessel!"
        FROM
            other_vessel_trips
    )
SELECT
    *
FROM
    other_vessels o
    INNER JOIN logged_in_user_values l ON TRUE
    INNER JOIN other_vessel_stats v ON TRUE
            "#,
            query.range.start(),
            query.range.end(),
            call_sign.as_ref(),
            &query.species_group_ids as &Option<Vec<SpeciesGroup>>,
            &vessels as &[FiskeridirVesselId],
            METERS_TO_NAUTICAL_MILES
        )
        .fetch_optional(&self.pool)
        .await?;

        let per_vessel: std::result::Result<Option<Vec<PerVesselStats>>, serde_json::Error> =
            if let Some(a) = &out
                && let Some(p) = &a.per_vessel
            {
                let stats: Vec<PerVesselStats> = serde_json::from_str(p)?;
                Ok(Some(stats))
            } else {
                Ok(None)
            };

        let per_vessel = per_vessel?.unwrap_or_default();

        Ok(out.map(|a| kyogre_core::AverageVesselsBenchmarks {
            lowest_fuel_consumption_liter: a.lowest_fuel_consumption_liter,
            highest_weight_per_hour: a.highest_weight_per_hour,
            highest_weight_per_distance: a.highest_weight_per_distance,
            highest_weight_per_fuel_liter: a.highest_weight_per_fuel_liter,
            highest_catch_value_per_fuel_liter: a.highest_catch_value_per_fuel_liter,
            highest_living_weight: a.highest_living_weight,
            own_trips_has_price_estimation: a.own_trips_has_price_estimation,
            own_fuel_measurement_coverage_percentage: a.own_fuel_measurement_coverage_percentage,
            own: kyogre_core::Stats {
                fuel_consumption_liter: a.own_sum_fuel_consumption_liter,
                weight_per_hour: a.own_sum_weight_per_hour,
                weight_per_distance: a.own_sum_weight_per_distance,
                weight_per_fuel_liter: a.own_sum_weight_per_fuel_liter,
                catch_value_per_fuel_liter: a.own_sum_catch_value_per_fuel_liter,
                living_weight: a.own_sum_living_weight,
                eeoi: own_eeoi,
                carbon_intensity: own_carbon_intensity,
                fui: own_carbon_intensity.map(|a| a / DIESEL_LITER_CARBON_FACTOR),
                num_trips: a.own_num_trips.map(|t| t as u64),
                num_hauls: a.own_num_hauls.map(|t| t as u64),
                value_nok: a.own_value_nok,
                trip_duration_seconds: a.own_trip_duration_seconds.map(|t| t as u64),
            },
            all: kyogre_core::Stats {
                eeoi: average_eeoi,
                carbon_intensity: average_carbon_intensity,
                fui: average_carbon_intensity.map(|a| a / DIESEL_LITER_CARBON_FACTOR),
                fuel_consumption_liter: a.average_fuel_consumption_liter,
                weight_per_hour: a.average_weight_per_hour,
                weight_per_distance: a.average_weight_per_distance,
                weight_per_fuel_liter: a.average_weight_per_fuel_liter,
                catch_value_per_fuel_liter: a.average_catch_value_per_fuel_liter,
                living_weight: a.average_living_weight,
                num_trips: a.average_num_trips.map(|t| t as u64),
                num_hauls: a.average_num_hauls.map(|t| t as u64),
                value_nok: a.average_value_nok,
                trip_duration_seconds: a.average_trip_duration_seconds.map(|t| t as u64),
            },
            per_vessel,
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
    t.benchmark_eeoi AS eeoi,
    t.benchmark_carbon_intensity AS carbon_intensity,
    t.benchmark_fui AS fui
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

    pub(crate) async fn carbon_intensity_impl(
        &self,
        query: &CarbonIntensityQuery,
    ) -> Result<Option<f64>> {
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
    END AS carbon_intensity
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
    AND (
        $6::INT[] IS NULL
        OR landing_two_highest_species_group_quantum && $6
    )
            "#,
            query.call_sign.as_ref(),
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            query.range.start(),
            query.range.end(),
            &query.species_group_ids as &Option<Vec<SpeciesGroup>>
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.carbon_intensity))
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
    AND (
        $7::INT[] IS NULL
        OR landing_two_highest_species_group_quantum && $7
    )
            "#,
            query.call_sign.as_ref(),
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            METERS_TO_NAUTICAL_MILES,
            query.range.start(),
            query.range.end(),
            &query.species_group_ids as &Option<Vec<SpeciesGroup>>
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.eeoi))
    }
    pub(crate) async fn average_carbon_intensity_impl(
        &self,
        call_sign: Option<&CallSign>,
        query: &AverageCarbonIntensityQuery,
    ) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"

WITH
    logged_in_vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            active_vessels
        WHERE
            call_sign = $1
            AND $1 IS NOT NULL
    ),
    carbon_intensities AS (
        SELECT
            CASE
                WHEN SUM(t.landing_total_living_weight) > 0
                AND SUM(t.distance) > $2 THEN (
                    SUM(
                        CASE
                            WHEN t.fiskeridir_vessel_id = (
                                SELECT
                                    fiskeridir_vessel_id
                                FROM
                                    logged_in_vessel_id
                            ) THEN t.benchmark_fuel_consumption_liter
                            ELSE t.benchmark_fuel_consumption_liter_estimated_only
                        END
                    ) * $3
                )::DOUBLE PRECISION / (
                    SUM(t.landing_total_living_weight)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
                )
                ELSE NULL
            END AS carbon_intensity
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
            carbon_intensity,
            percent_rank() OVER (
                ORDER BY
                    carbon_intensity
            ) AS percent
        FROM
            carbon_intensities
    )
SELECT
    AVG(carbon_intensity) AS carbon_intensity
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
            call_sign.map(|c| c.as_ref()),
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

        Ok(result.and_then(|v| v.carbon_intensity))
    }

    pub(crate) async fn average_eeoi_impl(
        &self,
        call_sign: Option<&CallSign>,
        query: &AverageEeoiQuery,
    ) -> Result<Option<f64>> {
        let result = sqlx::query!(
            r#"

WITH
    logged_in_vessel_id AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            active_vessels
        WHERE
            call_sign = $1
            AND $1 IS NOT NULL
    ),
    eeois AS (
        SELECT
            CASE
                WHEN SUM(t.landing_total_living_weight) > 0
                AND SUM(t.distance) > $2 THEN (
                    SUM(
                        CASE
                            WHEN t.fiskeridir_vessel_id = (
                                SELECT
                                    fiskeridir_vessel_id
                                FROM
                                    logged_in_vessel_id
                            ) THEN t.benchmark_fuel_consumption_liter
                            ELSE t.benchmark_fuel_consumption_liter_estimated_only
                        END
                    ) * $3
                )::DOUBLE PRECISION / (
                    SUM(t.landing_total_living_weight * t.distance * $4)::DOUBLE PRECISION / 1000::DOUBLE PRECISION
                )
                ELSE NULL
            END AS eeoi
        FROM
            trips_detailed t
        WHERE
            t.stop_timestamp BETWEEN $5 AND $6
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
            AND (
                $10::INT IS NULL
                OR t.landing_largest_quantum_species_group_id = $10
            )
            AND (
                $11::INT[] IS NULL
                OR landing_two_highest_species_group_quantum && $11
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
            call_sign.map(|c| c.as_ref()),
            MIN_EEOI_DISTANCE,
            DIESEL_LITER_CARBON_FACTOR,
            METERS_TO_NAUTICAL_MILES,
            query.range.start(),
            query.range.end(),
            query.length_group as Option<VesselLengthGroup>,
            query.gear_groups.as_slice().empty_to_none() as Option<&[GearGroup]>,
            query.vessel_ids.as_slice().empty_to_none() as Option<&[FiskeridirVesselId]>,
            query.species_group_id as Option<SpeciesGroup>,
            &query.species_group_ids as &Option<Vec<SpeciesGroup>>
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|v| v.eeoi))
    }
}
