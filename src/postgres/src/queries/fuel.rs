use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{BarentswatchUserId, FuelMeasurement, FuelMeasurementsQuery};

use crate::{
    error::Result,
    models::{DeleteFuelMeasurement, UpdateTripPositionFuel, UpsertFuelMeasurement},
    PostgresAdapter,
};

impl PostgresAdapter {
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
