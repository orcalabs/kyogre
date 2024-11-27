use chrono::NaiveDate;
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    BarentswatchUserId, FiskeridirVesselId, FuelMeasurement, FuelMeasurementsQuery, FuelQuery,
    Mmsi, NewFuelDayEstimate, ProcessingStatus,
};
use sqlx::postgres::types::PgRange;

use crate::{
    error::Result,
    models::{
        DeleteFuelMeasurement, UpdateTripPositionFuel, UpsertFuelEstimation, UpsertFuelMeasurement,
    },
    PostgresAdapter,
};

impl PostgresAdapter {
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
        ($1::DATERANGE)::DATEMULTIRANGE - COALESCE(RANGE_AGG(DATERANGE (date, date + 1, '[)')), '{}')::DATEMULTIRANGE
    ) AS "dates!"
FROM
    fuel_estimates
WHERE
    fiskeridir_vessel_id = $2
    AND status = $3
            "#,
            range,
            vessel_id.into_inner(),
            ProcessingStatus::Unprocessed as i32
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
        self.unnest_insert_from::<_, _, UpsertFuelEstimation>(estimates, &self.pool)
            .await
    }
    pub(crate) async fn fuel_estimation_impl(&self, query: &FuelQuery) -> Result<f64> {
        Ok(sqlx::query!(
            r#"
SELECT
    COALESCE(SUM(estimate), 0.0) AS "estimate!"
FROM
    fiskeridir_ais_vessel_mapping_whitelist w
    INNER JOIN fuel_estimates f ON w.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    call_sign = $1
    AND "date" >= $2
    AND "date" <= $3
            "#,
            query.call_sign.as_ref(),
            query.start_date,
            query.end_date,
        )
        .fetch_one(&self.pool)
        .await?
        .estimate)
    }
    pub(crate) async fn update_trip_position_fuel_consumption_impl(
        &self,
        values: &[kyogre_core::UpdateTripPositionFuel],
    ) -> Result<()> {
        self.unnest_update_from::<_, _, UpdateTripPositionFuel>(values, &self.pool)
            .await
    }
    pub(crate) fn fuel_measurements_impl(
        &self,
        query: FuelMeasurementsQuery,
    ) -> impl Stream<Item = Result<FuelMeasurement>> + '_ {
        sqlx::query_as!(
            FuelMeasurement,
            r#"
SELECT
    barentswatch_user_id AS "barentswatch_user_id!: BarentswatchUserId",
    call_sign AS "call_sign!: CallSign",
    timestamp,
    fuel
FROM
    fuel_measurements
WHERE
    barentswatch_user_id = $1
    AND call_sign = $2
    AND (
        $3::TIMESTAMPTZ IS NULL
        OR timestamp >= $3
    )
    AND (
        $4::TIMESTAMPTZ IS NULL
        OR timestamp <= $4
    )
ORDER BY
    timestamp DESC
            "#,
            query.barentswatch_user_id.as_ref(),
            query.call_sign.into_inner(),
            query.start_date,
            query.end_date,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn add_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::FuelMeasurement],
    ) -> Result<()> {
        self.unnest_insert_from::<_, _, UpsertFuelMeasurement<'_>>(measurements, &self.pool)
            .await
    }

    pub(crate) async fn update_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::FuelMeasurement],
    ) -> Result<()> {
        self.unnest_update_from::<_, _, UpsertFuelMeasurement<'_>>(measurements, &self.pool)
            .await
    }

    pub(crate) async fn delete_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::DeleteFuelMeasurement],
    ) -> Result<()> {
        self.unnest_delete_from::<_, _, DeleteFuelMeasurement<'_>>(measurements, &self.pool)
            .await
    }
}
