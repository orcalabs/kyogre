use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use fiskeridir_rs::DeliveryPointId;
use kyogre_core::{DeliveryPointSourceId, DeliveryPointType, TripId};

impl PostgresAdapter {
    pub(crate) async fn add_delivery_points<'a>(
        &'a self,
        delivery_points: Vec<DeliveryPointId>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let len = delivery_points.len();
        let mut delivery_point_ids = Vec::with_capacity(len);
        let mut delivery_point_types = Vec::with_capacity(len);
        let mut sources = Vec::with_capacity(len);

        for d in delivery_points {
            let delivery_point_type = if d.broenbaat() {
                DeliveryPointType::Broenbaat as i32
            } else {
                DeliveryPointType::Ukjent as i32
            };
            delivery_point_types.push(delivery_point_type);
            delivery_point_ids.push(d.into_inner());
            sources.push(DeliveryPointSourceId::NoteData as i32);
        }

        sqlx::query!(
            r#"
INSERT INTO
    delivery_points (
        delivery_point_id,
        delivery_point_type_id,
        delivery_point_source_id
    )
SELECT
    *
FROM
    UNNEST($1::VARCHAR[], $2::INT[], $3::INT[])
ON CONFLICT (delivery_point_id) DO NOTHING
            "#,
            delivery_point_ids.as_slice(),
            delivery_point_types.as_slice(),
            sources.as_slice(),
        )
        .execute(&mut **tx)
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
