use futures::TryStreamExt;
use std::collections::HashMap;

use crate::{error::ApiError, response::Response, *};
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use kyogre_core::{
    Delivery, FiskeridirVesselId, HaulId, Ordering, Pagination, TripId, Trips, VesselEventType,
};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use super::haul::Haul;

#[derive(Debug, Deserialize, IntoParams, Default)]
#[serde(rename_all = "camelCase")]
pub struct TripsParameters {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub ordering: Option<Ordering>,
}

#[utoipa::path(
    get,
    path = "/trip_of_haul/{haul_id}",
    responses(
        (status = 200, description = "trip associated with the given haul_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_of_haul<T: Database + 'static>(
    db: web::Data<T>,
    haul_id: Path<i64>,
) -> Result<Response<Option<Trip>>, ApiError> {
    db.detailed_trip_of_haul(&HaulId(haul_id.into_inner()))
        .await
        .map(|t| Response::new(t.map(Trip::from)))
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve trip of haul: {:?}", e);
            ApiError::InternalServerError
        })
}

#[utoipa::path(
    get,
    path = "/trips/{fiskeridir_vessel_id}",
    params(TripsParameters),
    responses(
        (status = 200, description = "trips of the given vessel", body = [Trip]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trips<T: Database + 'static>(
    db: web::Data<T>,
    fiskeridir_vessel_id: Path<u64>,
    params: web::Query<TripsParameters>,
) -> Result<HttpResponse, ApiError> {
    let params = params.into_inner();

    to_streaming_response! {
        db.detailed_trips_of_vessel(
            FiskeridirVesselId(fiskeridir_vessel_id.into_inner() as i64),
            Pagination::<Trips>::new(params.limit, params.offset),
            params.ordering.unwrap_or(Ordering::Asc)
        )
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve trips_of_vessel: {:?}", e);
            ApiError::InternalServerError
        })?
        .map_ok(Trip::from)
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve trips_of_vessel: {:?}", e);
            ApiError::InternalServerError
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    #[schema(value_type = i64)]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    #[schema(value_type = i64)]
    pub trip_id: TripId,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub num_deliveries: u32,
    pub most_recent_delivery_date: Option<DateTime<Utc>>,
    #[schema(value_type = Vec<u32>)]
    pub gear_ids: Vec<fiskeridir_rs::Gear>,
    #[schema(value_type = Vec<String>)]
    pub delivery_point_ids: Vec<fiskeridir_rs::DeliveryPointId>,
    pub hauls: Vec<Haul>,
    pub delivery: Delivery,
    #[schema(value_type = HashMap<String, Delivery>)]
    pub delivered_per_delivery_point: HashMap<fiskeridir_rs::DeliveryPointId, Delivery>,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub events: Vec<VesselEvent>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VesselEvent {
    pub event_id: u64,
    #[schema(value_type = i64)]
    pub event_type: VesselEventType,
    pub event_name: String,
    pub timestamp: DateTime<Utc>,
}

impl From<kyogre_core::TripDetailed> for Trip {
    fn from(value: kyogre_core::TripDetailed) -> Self {
        let (start, end) = if let Some(v) = value.period_precision {
            (v.start(), v.end())
        } else {
            (value.period.start(), value.period.end())
        };
        Trip {
            trip_id: value.trip_id,
            start,
            end,
            num_deliveries: value.num_deliveries,
            most_recent_delivery_date: value.most_recent_delivery_date,
            gear_ids: value.gear_ids,
            delivery_point_ids: value.delivery_point_ids,
            hauls: value.hauls.into_iter().map(Haul::from).collect(),
            delivery: value.delivery,
            delivered_per_delivery_point: value.delivered_per_delivery_point,
            start_port_id: value.start_port_id,
            end_port_id: value.end_port_id,
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            events: value
                .vessel_events
                .into_iter()
                .map(VesselEvent::from)
                .collect(),
        }
    }
}

impl From<kyogre_core::VesselEvent> for VesselEvent {
    fn from(value: kyogre_core::VesselEvent) -> Self {
        VesselEvent {
            event_id: value.event_id,
            event_type: value.event_type,
            event_name: value.event_type.name().to_owned(),
            timestamp: value.timestamp,
        }
    }
}

impl PartialEq<Trip> for kyogre_core::Trip {
    fn eq(&self, other: &Trip) -> bool {
        self.trip_id == other.trip_id
            && self.start().timestamp() == other.start.timestamp()
            && self.end().timestamp() == other.end.timestamp()
    }
}

impl PartialEq<Trip> for kyogre_core::TripDetailed {
    fn eq(&self, other: &Trip) -> bool {
        let converted: Trip = From::from(self.clone());
        converted.eq(other)
    }
}

impl PartialEq<kyogre_core::TripDetailed> for Trip {
    fn eq(&self, other: &kyogre_core::TripDetailed) -> bool {
        let converted: Trip = From::from(other.clone());
        converted.eq(self)
    }
}
