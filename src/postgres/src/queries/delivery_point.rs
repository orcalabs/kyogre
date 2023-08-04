use crate::{error::PostgresError, models::NewDeliveryPoint, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use fiskeridir_rs::DeliveryPointId;
use kyogre_core::TripId;
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_delivery_points<'a>(
        &'a self,
        delivery_points: Vec<DeliveryPointId>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let delivery_points = delivery_points
            .into_iter()
            .map(NewDeliveryPoint::from)
            .collect();

        NewDeliveryPoint::unnest_insert(delivery_points, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn delivery_points_associated_with_trip_impl(
        &self,
        _trip_id: TripId,
    ) -> Result<Vec<kyogre_core::DeliveryPoint>, PostgresError> {
        // TODO: implement when we have coordinates for delivery points
        Ok(vec![])
    }
}
