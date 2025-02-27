use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    DateRange, FiskeridirVesselId, FuelQuery, LiveFuelQuery, LiveFuelVessel, Mmsi,
    NewFuelDayEstimate, NewLiveFuel, ProcessingStatus,
};
use sqlx::postgres::types::PgRange;
use unnest_insert::UnnestInsert;

use crate::{PostgresAdapter, error::Result, models::UpsertNewLiveFuel};

impl PostgresAdapter {
    pub(crate) fn live_fuel_impl(
        &self,
        query: &LiveFuelQuery,
    ) -> impl Stream<Item = Result<kyogre_core::LiveFuelEntry>> + '_ {
        sqlx::query_as!(
            kyogre_core::LiveFuelEntry,
            r#"
SELECT
    COALESCE(SUM(fuel_liter), 0.0) AS "fuel_liter!",
    DATE_TRUNC('hour', f.latest_position_timestamp) AS "timestamp!"
FROM
    active_vessels w
    INNER JOIN live_fuel f ON w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    w.call_sign = $1
    AND truncate_ts_to_hour (f.latest_position_timestamp) >= $2
GROUP BY
    f.fiskeridir_vessel_id,
    f.year,
    f.day,
    f.hour
            "#,
            query.call_sign.as_ref(),
            query.threshold
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn delete_old_live_fuel_impl(
        &self,
        fiskeridir_vessel_id: FiskeridirVesselId,
        threshold: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
DELETE FROM live_fuel
WHERE
    fiskeridir_vessel_id = $1
    AND latest_position_timestamp <= $2
            "#,
            fiskeridir_vessel_id.into_inner(),
            threshold,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn live_fuel_vessels_impl(&self) -> Result<Vec<LiveFuelVessel>> {
        Ok(sqlx::query_as!(
            LiveFuelVessel,
            r#"
SELECT
    w.mmsi AS "mmsi!: Mmsi",
    f.fiskeridir_vessel_id AS "vessel_id!: FiskeridirVesselId",
    f.engine_building_year_final AS "engine_building_year!",
    f.engine_power_final AS "engine_power!",
    f.auxiliary_engine_power AS "auxiliary_engine_power?",
    f.auxiliary_engine_building_year AS "auxiliary_engine_building_year?",
    f.boiler_engine_power AS "boiler_engine_power?",
    f.boiler_engine_building_year AS "boiler_engine_building_year?",
    f.service_speed AS "service_speed?",
    f.degree_of_electrification AS "degree_of_electrification?",
    t.departure_timestamp AS "current_trip_start?",
    -- Hacky fix because sqlx prepare/check flakes on nullability
    COALESCE(q.latest_position_timestamp, NULL) AS latest_position_timestamp
FROM
    active_vessels w
    INNER JOIN fiskeridir_vessels f ON w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN current_trips t ON t.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN (
        SELECT DISTINCT
            ON (fiskeridir_vessel_id) fiskeridir_vessel_id,
            latest_position_timestamp
        FROM
            live_fuel
        ORDER BY
            fiskeridir_vessel_id,
            latest_position_timestamp DESC
    ) q ON q.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    w.mmsi IS NOT NULL
    AND f.engine_building_year_final IS NOT NULL
    AND f.engine_power_final IS NOT NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn add_live_fuel_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        fuel: &[NewLiveFuel],
    ) -> Result<()> {
        UnnestInsert::unnest_insert(
            fuel.iter()
                .map(|f| UpsertNewLiveFuel::from_core(vessel_id, f)),
            &self.pool,
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn reset_fuel_estimation(
        &self,
        vessel_id: FiskeridirVesselId,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE fuel_estimates
SET
    status = $1
WHERE
    fiskeridir_vessel_id = $2
            "#,
            ProcessingStatus::Unprocessed as i32,
            vessel_id.into_inner()
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn dates_to_estimate_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        call_sign: Option<&CallSign>,
        mmsi: Option<Mmsi>,
        end_date: NaiveDate,
    ) -> Result<Vec<NaiveDate>> {
        let earliest_position = self.earliest_position_impl(call_sign, mmsi).await?;

        let Some(earliest_position) = earliest_position else {
            return Ok(vec![]);
        };

        let range = PgRange {
            start: std::ops::Bound::Included(earliest_position),
            end: std::ops::Bound::Included(end_date),
        };

        sqlx::query!(
            r#"
SELECT
    UNNEST(
        ($1::DATERANGE)::DATEMULTIRANGE - COALESCE(
            RANGE_AGG(DATERANGE (date::DATE, date::DATE + 1, '[)')),
            '{}'
        )::DATEMULTIRANGE
    ) AS "dates!"
FROM
    fuel_estimates
WHERE
    fiskeridir_vessel_id = $2
    AND status = $3
            "#,
            range,
            vessel_id.into_inner(),
            ProcessingStatus::Successful as i32
        )
        .fetch(&self.pool)
        .map_ok(|r| {
            let mut current = match r.dates.start {
                std::ops::Bound::Included(t) => t,
                std::ops::Bound::Excluded(t) => t.succ_opt().unwrap(),
                std::ops::Bound::Unbounded => unreachable!(),
            };
            let end = match r.dates.end {
                std::ops::Bound::Included(t) => t,
                std::ops::Bound::Excluded(t) => t.pred_opt().unwrap(),
                std::ops::Bound::Unbounded => unreachable!(),
            };

            let mut out = Vec::new();
            while current <= end {
                out.push(current);
                current = current.succ_opt().unwrap();
            }
            out
        })
        .map_err(|e| e.into())
        .try_concat()
        .await
    }

    pub(crate) async fn add_fuel_estimates_impl(
        &self,
        estimates: &[NewFuelDayEstimate],
    ) -> Result<()> {
        let len = estimates.len();
        let mut vessel_id = Vec::with_capacity(len);
        let mut engine_version = Vec::with_capacity(len);
        let mut date = Vec::with_capacity(len);
        let mut estimate_liter = Vec::with_capacity(len);
        let mut num_ais_positions = Vec::with_capacity(len);
        let mut num_vms_positions = Vec::with_capacity(len);

        for e in estimates {
            vessel_id.push(e.vessel_id.into_inner());
            engine_version.push(e.engine_version as i32);
            date.push(e.date);
            estimate_liter.push(e.estimate_liter);
            num_ais_positions.push(e.num_ais_positions as i32);
            num_vms_positions.push(e.num_vms_positions as i32);
        }

        sqlx::query!(
            r#"
INSERT INTO
    fuel_estimates (
        fiskeridir_vessel_id,
        date,
        estimate_liter,
        num_ais_positions,
        num_vms_positions,
        status
    )
SELECT
    u.id,
    u.date,
    u.estimate_liter,
    u.num_ais_positions,
    u.num_vms_positions,
    $7
FROM
    fiskeridir_vessels f
    INNER JOIN UNNEST(
        $1::BIGINT[],
        $2::INT[],
        $3::DATE[],
        $4::DOUBLE PRECISION[],
        $5::INT[],
        $6::INT[]
    ) u (
        id,
        engine_version,
        date,
        estimate_liter,
        num_ais_positions,
        num_vms_positions
    ) ON u.id = f.fiskeridir_vessel_id
    AND u.engine_version = f.engine_version
ON CONFLICT (fiskeridir_vessel_id, date) DO UPDATE
SET
    estimate_liter = EXCLUDED.estimate_liter,
    num_ais_positions = EXCLUDED.num_ais_positions,
    num_vms_positions = EXCLUDED.num_vms_positions,
    status = EXCLUDED.status
            "#,
            &vessel_id,
            &engine_version,
            &date,
            &estimate_liter,
            &num_ais_positions,
            &num_vms_positions,
            ProcessingStatus::Successful as i32
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn fuel_estimation_impl(&self, query: &FuelQuery) -> Result<f64> {
        let range = DateRange::from_dates(query.start_date, query.end_date)?;
        let pg_range: PgRange<DateTime<Utc>> = (&range).into();

        Ok(sqlx::query!(
            r#"
WITH
    vessels AS (
        SELECT
            fiskeridir_vessel_id
        FROM
            active_vessels
        WHERE
            call_sign = $1
    ),
    measurements AS (
        SELECT
            SUM(
                COMPUTE_TS_RANGE_PERCENT_OVERLAP (r.fuel_range, $2) * r.fuel_used_liter
            ) AS fuel_used_liter,
            RANGE_AGG(r.fuel_range) AS fuel_ranges
        FROM
            vessels v
            INNER JOIN fuel_measurement_ranges r ON v.fiskeridir_vessel_id = r.fiskeridir_vessel_id
            AND r.fuel_range && $2
            AND COMPUTE_TS_RANGE_PERCENT_OVERLAP (r.fuel_range, $2) >= 0.5
    ),
    overlapping AS (
        SELECT
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
    )
SELECT
    COALESCE(SUM(q.fuel_liter), 0.0) AS "estimate_liter!"
FROM
    (
        SELECT
            fuel_liter
        FROM
            overlapping
        UNION ALL
        SELECT
            fuel_used_liter AS fuel_liter
        FROM
            measurements
    ) q
            "#,
            query.call_sign.as_ref(),
            pg_range,
        )
        .fetch_one(&self.pool)
        .await?
        .estimate_liter)
    }

    pub(crate) async fn delete_fuel_estimates_impl(
        &self,
        vessels: &[FiskeridirVesselId],
    ) -> Result<()> {
        sqlx::query!(
            r#"
DELETE FROM fuel_estimates
WHERE
    fiskeridir_vessel_id = ANY ($1::BIGINT[])
            "#,
            vessels as &[FiskeridirVesselId],
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| e.into())
    }
}
