use crate::{error::Result, to_streaming_response, Database};
use actix_web::{web, HttpResponse};
use fiskeridir_rs::DeliveryPointId;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/delivery_points",
    responses(
        (status = 200, description = "all delivery points", body = [DeliveryPoint]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn delivery_points<T: Database + 'static>(db: web::Data<T>) -> Result<HttpResponse> {
    to_streaming_response! {
        db.delivery_points().map_ok(DeliveryPoint::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryPoint {
    #[schema(value_type = String)]
    pub id: DeliveryPointId,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

impl From<kyogre_core::DeliveryPoint> for DeliveryPoint {
    fn from(v: kyogre_core::DeliveryPoint) -> Self {
        DeliveryPoint {
            id: v.id,
            name: v.name,
            address: v.address,
            latitude: v.latitude,
            longitude: v.longitude,
        }
    }
}

impl From<fiskeridir_rs::AquaCultureEntry> for DeliveryPoint {
    fn from(v: fiskeridir_rs::AquaCultureEntry) -> Self {
        DeliveryPoint {
            id: v.delivery_point_id,
            name: Some(v.name),
            address: v.address,
            latitude: Some(v.latitude),
            longitude: Some(v.longitude),
        }
    }
}

impl PartialEq<kyogre_core::DeliveryPoint> for DeliveryPoint {
    fn eq(&self, other: &kyogre_core::DeliveryPoint) -> bool {
        let other: DeliveryPoint = other.clone().into();
        self.eq(&other)
    }
}

impl PartialEq<DeliveryPoint> for kyogre_core::DeliveryPoint {
    fn eq(&self, other: &DeliveryPoint) -> bool {
        other.eq(self)
    }
}

impl PartialEq<fiskeridir_rs::AquaCultureEntry> for DeliveryPoint {
    fn eq(&self, other: &fiskeridir_rs::AquaCultureEntry) -> bool {
        let other: DeliveryPoint = other.clone().into();
        self.eq(&other)
    }
}

impl PartialEq<DeliveryPoint> for fiskeridir_rs::AquaCultureEntry {
    fn eq(&self, other: &DeliveryPoint) -> bool {
        other.eq(self)
    }
}
