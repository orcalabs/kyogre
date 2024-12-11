use actix_web::web;
use fiskeridir_rs::DeliveryPointId;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ErrorResponse, response::StreamResponse, stream_response, Database};

/// Returns all known delivery points.
/// Delivery points originates from the following sources:
/// - Buyer register from Fiskeridirektoratet
/// - Aqua culture register from Fiskeridirektoratet
/// - Mattilsynet approval lists
/// - Manual entries
#[utoipa::path(
    get,
    path = "/delivery_points",
    responses(
        (status = 200, description = "all delivery points", body = [DeliveryPoint]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn delivery_points<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<DeliveryPoint> {
    stream_response! {
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
        let kyogre_core::DeliveryPoint {
            id,
            name,
            address,
            latitude,
            longitude,
        } = v;

        DeliveryPoint {
            id,
            name,
            address,
            latitude,
            longitude,
        }
    }
}

impl From<fiskeridir_rs::AquaCultureEntry> for DeliveryPoint {
    fn from(v: fiskeridir_rs::AquaCultureEntry) -> Self {
        let fiskeridir_rs::AquaCultureEntry {
            till_nr: _,
            org_number: _,
            name,
            address,
            zip_code: _,
            city: _,
            approval_date: _,
            approval_limit: _,
            till_municipality_number: _,
            till_municipality: _,
            purpose: _,
            production_form: _,
            species: _,
            species_code: _,
            till_kap: _,
            till_unit: _,
            delivery_point_id,
            locality_name: _,
            locality_municipality_number: _,
            locality_municipality: _,
            locality_location: _,
            water_environment: _,
            locality_kap: _,
            locality_unit: _,
            expiration_date: _,
            latitude: _,
            longitude: _,
            prod_omr: _,
        } = v;

        DeliveryPoint {
            id: delivery_point_id,
            name: Some(name.into_inner()),
            address: address.map(|v| v.into_inner()),
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
