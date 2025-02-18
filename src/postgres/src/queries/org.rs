use crate::{error::Result, models::OrgBenchmarks, PostgresAdapter};
use chrono::{DateTime, Utc};
use fiskeridir_rs::OrgId;
use kyogre_core::{DateRange, FiskeridirVesselId, FuelEntry, FuelQuery, OrgBenchmarkQuery};
use sqlx::postgres::types::PgRange;

impl PostgresAdapter {
    pub(crate) async fn fuel_estimation_by_org_impl(
        &self,
        query: &FuelQuery,
        org_id: OrgId,
    ) -> Result<Option<Vec<FuelEntry>>> {
        let Some(org_vessels) = self
            .assert_call_sign_is_in_org(&query.call_sign, org_id)
            .await?
        else {
            return Ok(None);
        };

        let range = DateRange::from_dates(query.start_date, query.end_date)?;
        let pg_range: PgRange<DateTime<Utc>> = (&range).into();

        let values =
                    sqlx::query_as!(
                        FuelEntry,
                        r#"
WITH
    vessels AS (
        SELECT
            a.fiskeridir_vessel_id
        FROM
            unnest($1::BIGINT[]) a (fiskeridir_vessel_id)
    ),
    measurements AS (
        SELECT
            v.fiskeridir_vessel_id,
            SUM(
                COMPUTE_TS_RANGE_PERCENT_OVERLAP (r.fuel_range, $2) * r.fuel_used_liter
            ) AS fuel_used_liter,
            RANGE_AGG(r.fuel_range) AS fuel_ranges
        FROM
            vessels v
            INNER JOIN fuel_measurement_ranges r ON v.fiskeridir_vessel_id = r.fiskeridir_vessel_id
            AND r.fuel_range && $2
            AND COMPUTE_TS_RANGE_PERCENT_OVERLAP (r.fuel_range, $2) >= 0.5
        GROUP BY
            v.fiskeridir_vessel_id
    ),
    overlapping AS (
        SELECT
            v.fiskeridir_vessel_id,
            SUM(
                CASE
                    WHEN m.fuel_ranges IS NULL THEN f.estimate_liter
                    ELSE (
                        1.0 - COMPUTE_TS_RANGE_MUTLIRANGE_PERCENT_OVERLAP (f.day_range, m.fuel_ranges)
                    ) * f.estimate_liter
                END
            ) AS fuel_liter
        FROM
            vessels v
            INNER JOIN fuel_estimates f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
            AND f.day_range <@ $2
            LEFT JOIN measurements m ON m.fuel_ranges && f.day_range
            AND m.fiskeridir_vessel_id = f.fiskeridir_vessel_id
        GROUP BY
            v.fiskeridir_vessel_id
    )
SELECT
    q.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    COALESCE(SUM(q.fuel_liter), 0.0) AS "estimated_fuel_liter!"
FROM
    (
        SELECT
            fiskeridir_vessel_id,
            fuel_liter
        FROM
            overlapping
        UNION ALL
        SELECT
            fiskeridir_vessel_id,
            fuel_used_liter AS fuel_liter
        FROM
            measurements
    ) q
GROUP BY
    q.fiskeridir_vessel_id
                        "#,
                        &org_vessels as &[FiskeridirVesselId],
                        pg_range,
                    )
                    .fetch_all(&self.pool)
                    .await?;

        Ok(Some(values))
    }
    pub(crate) async fn org_benchmarks_impl(
        &self,
        query: &OrgBenchmarkQuery,
    ) -> Result<Option<OrgBenchmarks>> {
        let Some(org_vessels) = self
            .assert_call_sign_is_in_org(&query.call_sign, query.org_id)
            .await?
        else {
            return Ok(None);
        };

        let benchmark = sqlx::query_as!(
            OrgBenchmarks,
            r#"
WITH
    vessels AS (
        SELECT
            a.fiskeridir_vessel_id
        FROM
            unnest($1::BIGINT[]) a (fiskeridir_vessel_id)
    ),
    trips AS (
        SELECT
            v.fiskeridir_vessel_id,
            $2::BIGINT AS org_id,
            SUM(haul_duration) AS haul_duration,
            SUM(distance) AS distance,
            SUM(trip_duration) AS trip_duration,
            SUM(landing_total_living_weight) AS landing_total_living_weight,
            SUM(landing_total_price_for_fisher) AS price_for_fisher,
            ARRAY_CONCAT (landing_ids) FILTER (
                WHERE
                    landing_ids IS NOT NULL
                    AND CARDINALITY(landing_ids) > 0
            ) AS landing_ids
        FROM
            vessels v
            LEFT JOIN trips_detailed t ON t.fiskeridir_vessel_id = v.fiskeridir_vessel_id
            AND t.start_timestamp >= $3
            AND t.stop_timestamp <= $4
        GROUP BY
            v.fiskeridir_vessel_id
    )
SELECT
    COALESCE(
        EXTRACT(
            'epoch'
            FROM
                SUM(q.haul_duration)
        ),
        0
    )::BIGINT AS "fishing_time!",
    COALESCE(SUM(q.distance), 0.0)::DOUBLE PRECISION AS "trip_distance!",
    COALESCE(
        EXTRACT(
            'epoch'
            FROM
                SUM(q.trip_duration)
        ),
        0
    )::BIGINT AS "trip_time!",
    COALESCE(SUM(q.landing_total_living_weight), 0.0)::DOUBLE PRECISION AS "landing_total_living_weight!",
    COALESCE(SUM(q.price_for_fisher), 0.0)::DOUBLE PRECISION AS "price_for_fisher!",
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'fiskeridir_vessel_id',
                q.fiskeridir_vessel_id,
                'fishing_time',
                COALESCE(
                    EXTRACT(
                        'epoch'
                        FROM
                            q.haul_duration
                    ),
                    0
                )::BIGINT,
                'trip_distance',
                COALESCE(q.distance, 0.0)::DOUBLE PRECISION,
                'trip_time',
                COALESCE(
                    EXTRACT(
                        'epoch'
                        FROM
                            q.trip_duration
                    ),
                    0
                )::BIGINT,
                'landing_total_living_weight',
                COALESCE(q.landing_total_living_weight, 0.0)::DOUBLE PRECISION,
                'price_for_fisher',
                COALESCE(q.price_for_fisher, 0.0)::DOUBLE PRECISION,
                'species',
                COALESCE(q.species, '[]')::JSONB
            )
        ),
        '[]'
    )::TEXT AS "vessels!"
FROM
    (
        SELECT
            t.fiskeridir_vessel_id,
            MAX(t.org_id) AS org_id,
            MAX(t.haul_duration) AS haul_duration,
            MAX(t.distance) AS distance,
            MAX(t.trip_duration) AS trip_duration,
            MAX(t.landing_total_living_weight) AS landing_total_living_weight,
            MAX(t.price_for_fisher) AS price_for_fisher,
            JSONB_AGG(
                JSONB_BUILD_OBJECT(
                    'species_group_id',
                    q.species_group_id,
                    'landing_total_living_weight',
                    q.living_weight,
                    'price_for_fisher',
                    q.price_for_fisher
                )
                ORDER BY
                    q.species_group_id,
                    q.living_weight
            ) FILTER (
                WHERE
                    q.species_group_id IS NOT NULL
            ) AS species
        FROM
            trips t
            LEFT JOIN (
                SELECT
                    l.species_group_id,
                    t.fiskeridir_vessel_id,
                    COALESCE(SUM(l.living_weight), 0.0)::DOUBLE PRECISION AS living_weight,
                    COALESCE(SUM(l.final_price_for_fisher), 0.0)::DOUBLE PRECISION AS price_for_fisher
                FROM
                    trips t
                    INNER JOIN landing_entries l ON l.landing_id = ANY (t.landing_ids)
                GROUP BY
                    t.fiskeridir_vessel_id,
                    l.species_group_id
            ) q ON q.fiskeridir_vessel_id = t.fiskeridir_vessel_id
        GROUP BY
            t.fiskeridir_vessel_id
    ) q
GROUP BY
    q.org_id
            "#,
            &org_vessels as &[FiskeridirVesselId],
            query.org_id.into_inner(),
            query.start,
            query.end,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(benchmark)
    }
}
