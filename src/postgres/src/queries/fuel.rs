use crate::{
    error::Result,
    models::{DeleteFuelMeasurement, FuelMeasurement},
    PostgresAdapter,
};
use futures::{Stream, TryStreamExt};
use kyogre_core::FuelMeasurementsQuery;
use unnest_insert::{UnnestDelete, UnnestInsert, UnnestUpdate};

impl PostgresAdapter {
    pub(crate) fn fuel_measurements_impl(
        &self,
        query: FuelMeasurementsQuery,
    ) -> impl Stream<Item = Result<FuelMeasurement>> + '_ {
        sqlx::query_as!(
            FuelMeasurement,
            r#"
SELECT
    barentswatch_user_id,
    call_sign,
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
            query.barentswatch_user_id.0,
            query.call_sign.into_inner(),
            query.start_date,
            query.end_date,
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }

    pub(crate) async fn add_fuel_measurements_impl(
        &self,
        measurements: Vec<kyogre_core::FuelMeasurement>,
    ) -> Result<()> {
        let values = measurements.into_iter().map(From::from).collect();
        FuelMeasurement::unnest_insert(values, &self.pool).await?;
        Ok(())
    }

    pub(crate) async fn update_fuel_measurements_impl(
        &self,
        measurements: Vec<kyogre_core::FuelMeasurement>,
    ) -> Result<()> {
        let values = measurements.into_iter().map(From::from).collect();
        FuelMeasurement::unnest_update(values, &self.pool).await?;
        Ok(())
    }

    pub(crate) async fn delete_fuel_measurements_impl(
        &self,
        measurements: Vec<kyogre_core::DeleteFuelMeasurement>,
    ) -> Result<()> {
        let values = measurements.into_iter().map(From::from).collect();
        DeleteFuelMeasurement::unnest_delete(values, &self.pool).await?;
        Ok(())
    }
}
