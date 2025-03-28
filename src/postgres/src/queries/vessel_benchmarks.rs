use crate::{PostgresAdapter, error::Result, models::VesselBenchmarks};
use chrono::{Datelike, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{BarentswatchUserId, FiskeridirVesselId, ProcessingStatus};

impl PostgresAdapter {
    pub(crate) async fn reset_bencmarks(
        &self,
        vessel_id: FiskeridirVesselId,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE trips_detailed o
SET
    benchmark_status = $1
FROM
    trips_detailed t
WHERE
    t.fiskeridir_vessel_id = $2
    AND t.trip_id = o.trip_id
                "#,
            ProcessingStatus::Unprocessed as i32,
            vessel_id.into_inner()
        )
        .execute(executor)
        .await?;
        Ok(())
    }
    pub(crate) async fn vessel_benchmarks_impl(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
    ) -> Result<VesselBenchmarks> {
        let year = Utc::now().year();

        Ok(sqlx::query_as!(
            VesselBenchmarks,
            r#"
SELECT
    COALESCE(
        (
            SELECT
                JSONB_AGG(
                    t.cumulative_sums
                    ORDER BY
                        t.cumulative_sums ->> 'month',
                        t.cumulative_sums ->> 'species_fiskeridir_id' ASC
                )
            FROM
                (
                    SELECT
                        JSONB_BUILD_OBJECT(
                            'month',
                            r.month::INT,
                            'species_fiskeridir_id',
                            r.species_fiskeridir_id,
                            'weight',
                            r.weight,
                            'cumulative_weight',
                            SUM(r.weight) OVER (
                                PARTITION BY
                                    r.species_fiskeridir_id
                                ORDER BY
                                    r.species_fiskeridir_id,
                                    r.month ASC ROWS BETWEEN UNBOUNDED PRECEDING
                                    AND CURRENT ROW
                            )
                        ) AS cumulative_sums
                    FROM
                        (
                            SELECT
                                DATE_PART('month', landing_timestamp) AS "month",
                                SUM(living_weight) AS weight,
                                le.species_fiskeridir_id
                            FROM
                                active_vessels f
                                INNER JOIN landings l ON l.fiskeridir_vessel_id = f.fiskeridir_vessel_id
                                INNER JOIN landing_entries le ON le.landing_id = l.landing_id
                            WHERE
                                call_sign = $1
                                AND DATE_PART('year', landing_timestamp)::INT = $3
                            GROUP BY
                                DATE_PART('month', landing_timestamp),
                                le.species_fiskeridir_id
                        ) r
                ) t
        ),
        '[]'
    )::TEXT AS "cumulative_landings!",
    JSONB_BUILD_OBJECT(
        'average',
        COALESCE(
            AVG(
                EXTRACT(
                    epoch
                    FROM
                        q.haul_duration
                ) / 60
            ) FILTER (
                WHERE
                    q.is_self IS TRUE
            ),
            0
        ),
        'averageFollowers',
        COALESCE(
            AVG(
                EXTRACT(
                    epoch
                    FROM
                        q.haul_duration
                ) / 60
            ) FILTER (
                WHERE
                    q.is_self IS FALSE
            ),
            0
        ),
        'recentTrips',
        COALESCE(
            JSONB_AGG(
                q.trip_haul_duration_json
                ORDER BY
                    q.trip_haul_duration_json ->> 'tripStart',
                    q.trip_haul_duration_json ->> 'fiskeridirVesselId'
            ) FILTER (
                WHERE
                    q.trip_haul_duration_json ->> 'value' IS NOT NULL
            ),
            '[]'
        )
    )::TEXT AS fishing_time,
    JSONB_BUILD_OBJECT(
        'average',
        COALESCE(
            AVG(q.trip_distance) FILTER (
                WHERE
                    q.is_self IS TRUE
            ),
            0
        ),
        'averageFollowers',
        COALESCE(
            AVG(q.trip_distance) FILTER (
                WHERE
                    q.is_self IS FALSE
            ),
            0
        ),
        'recentTrips',
        COALESCE(
            JSONB_AGG(
                q.trip_distance_json
                ORDER BY
                    q.trip_distance_json ->> 'tripStart',
                    q.trip_distance_json ->> 'fiskeridirVesselId'
            ) FILTER (
                WHERE
                    q.trip_distance_json ->> 'value' IS NOT NULL
            ),
            '[]'
        )
    )::TEXT AS fishing_distance,
    JSONB_BUILD_OBJECT(
        'average',
        COALESCE(
            AVG(
                EXTRACT(
                    epoch
                    FROM
                        q.trip_duration
                ) / 60
            ) FILTER (
                WHERE
                    q.is_self IS TRUE
            ),
            0
        ),
        'averageFollowers',
        COALESCE(
            AVG(
                EXTRACT(
                    epoch
                    FROM
                        q.trip_duration
                ) / 60
            ) FILTER (
                WHERE
                    q.is_self IS FALSE
            ),
            0
        ),
        'recentTrips',
        COALESCE(
            JSONB_AGG(
                q.trip_duration_json
                ORDER BY
                    q.trip_duration_json ->> 'tripStart',
                    q.trip_duration_json ->> 'fiskeridirVesselId'
            ) FILTER (
                WHERE
                    q.trip_duration_json ->> 'value' IS NOT NULL
            ),
            '[]'
        )
    )::TEXT AS trip_time,
    JSONB_BUILD_OBJECT(
        'average',
        COALESCE(
            AVG(q.landing_total_living_weight) FILTER (
                WHERE
                    q.is_self IS TRUE
            ),
            0
        ),
        'averageFollowers',
        COALESCE(
            AVG(q.landing_total_living_weight) FILTER (
                WHERE
                    q.is_self IS FALSE
            ),
            0
        ),
        'recentTrips',
        COALESCE(
            JSONB_AGG(
                q.trip_landing_weight_json
                ORDER BY
                    q.trip_landing_weight_json ->> 'tripStart',
                    q.trip_landing_weight_json ->> 'fiskeridirVesselId'
            ) FILTER (
                WHERE
                    q.trip_landing_weight_json ->> 'value' IS NOT NULL
            ),
            '[]'
        )
    )::TEXT AS landings,
    JSONB_BUILD_OBJECT(
        'average',
        COALESCE(
            AVG(q.haul_total_weight) FILTER (
                WHERE
                    q.is_self IS TRUE
            ),
            0
        ),
        'averageFollowers',
        COALESCE(
            AVG(q.haul_total_weight) FILTER (
                WHERE
                    q.is_self IS FALSE
            ),
            0
        ),
        'recentTrips',
        COALESCE(
            JSONB_AGG(
                q.trip_haul_weight_json
                ORDER BY
                    q.trip_haul_weight_json ->> 'tripStart',
                    q.trip_haul_weight_json ->> 'fiskeridirVesselId'
            ) FILTER (
                WHERE
                    q.trip_haul_weight_json ->> 'value' IS NOT NULL
            ),
            '[]'
        )
    )::TEXT AS ers_dca
FROM
    (
        SELECT
            MAX(k.fiskeridir_vessel_id) AS fiskeridir_vessel_id,
            MAX(k.trip_start) AS trip_start,
            MAX(k.trip_distance) AS trip_distance,
            MAX(k.landing_total_living_weight) AS landing_total_living_weight,
            MAX(k.haul_duration) AS haul_duration,
            MAX(k.trip_duration) AS trip_duration,
            MAX(k.haul_total_weight) AS haul_total_weight,
            (ARRAY_AGG(k.is_self)) [1] AS is_self,
            JSONB_BUILD_OBJECT(
                'fiskeridirVesselId',
                MAX(k.fiskeridir_vessel_id),
                'tripStart',
                MAX(k.trip_start),
                'value',
                MAX(
                    EXTRACT(
                        epoch
                        FROM
                            k.haul_duration
                    ) / 60
                )
            ) AS trip_haul_duration_json,
            JSONB_BUILD_OBJECT(
                'fiskeridirVesselId',
                MAX(k.fiskeridir_vessel_id),
                'tripStart',
                MAX(k.trip_start),
                'value',
                MAX(k.trip_distance)
            ) AS trip_distance_json,
            JSONB_BUILD_OBJECT(
                'fiskeridirVesselId',
                MAX(k.fiskeridir_vessel_id),
                'tripStart',
                MAX(k.trip_start),
                'value',
                MAX(
                    EXTRACT(
                        epoch
                        FROM
                            k.trip_duration
                    ) / 60
                )
            ) AS trip_duration_json,
            JSONB_BUILD_OBJECT(
                'fiskeridirVesselId',
                MAX(k.fiskeridir_vessel_id),
                'tripStart',
                MAX(k.trip_start),
                'value',
                MAX(k.landing_total_living_weight)
            ) AS trip_landing_weight_json,
            JSONB_BUILD_OBJECT(
                'fiskeridirVesselId',
                MAX(k.fiskeridir_vessel_id),
                'tripStart',
                MAX(k.trip_start),
                'value',
                MAX(k.haul_total_weight)
            ) AS trip_haul_weight_json
        FROM
            (
                SELECT
                    qi.fiskeridir_vessel_id,
                    qi.is_self,
                    td.trip_id,
                    td.distance AS trip_distance,
                    td.landing_total_living_weight,
                    td.haul_duration,
                    td.trip_duration,
                    td.haul_total_weight,
                    LOWER(td.period) AS trip_start,
                    ROW_NUMBER() OVER (
                        PARTITION BY
                            td.fiskeridir_vessel_id
                        ORDER BY
                            td.period DESC
                    ) AS r
                FROM
                    (
                        SELECT
                            fiskeridir_vessel_id,
                            TRUE AS is_self
                        FROM
                            active_vessels f
                        WHERE
                            f.call_sign = $1
                        UNION
                        SELECT
                            fiskeridir_vessel_id,
                            FALSE AS is_self
                        FROM
                            user_follows uf
                        WHERE
                            uf.barentswatch_user_id = $2
                    ) qi
                    INNER JOIN trips_detailed td ON qi.fiskeridir_vessel_id = td.fiskeridir_vessel_id
            ) k
        WHERE
            k.r <= 10
        GROUP BY
            k.trip_id
    ) q
            "#,
            call_sign.as_ref(),
            user_id.as_ref(),
            year,
        )
        .fetch_one(&self.pool)
        .await?)
    }
}
