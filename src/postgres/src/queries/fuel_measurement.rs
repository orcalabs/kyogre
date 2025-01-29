use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{BarentswatchUserId, FuelMeasurement, FuelMeasurementId, FuelMeasurementsQuery};

use crate::{
    error::Result,
    models::{CreateFuelMeasurement, DeleteFuelMeasurement, UpdateFuelMeasurement},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn fuel_measurements_impl(
        &self,
        query: FuelMeasurementsQuery,
    ) -> impl Stream<Item = Result<FuelMeasurement>> + '_ {
        sqlx::query_as!(
            FuelMeasurement,
            r#"
SELECT
    fuel_measurements_id AS "id!: FuelMeasurementId",
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
        measurements: &[kyogre_core::CreateFuelMeasurement],
    ) -> Result<Vec<kyogre_core::FuelMeasurement>> {
        self.unnest_insert_from_returning::<_, _, CreateFuelMeasurement<'_>>(
            measurements,
            &self.pool,
        )
        .map_ok(|v| kyogre_core::FuelMeasurement {
            id: v.fuel_measurements_id,
            barentswatch_user_id: v.barentswatch_user_id,
            call_sign: v.call_sign,
            timestamp: v.timestamp,
            fuel: v.fuel,
        })
        .try_collect()
        .await
    }

    pub(crate) async fn update_fuel_measurements_impl(
        &self,
        measurements: &[kyogre_core::FuelMeasurement],
    ) -> Result<()> {
        self.unnest_update_from::<_, _, UpdateFuelMeasurement<'_>>(measurements, &self.pool)
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
