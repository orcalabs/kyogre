use crate::routes::utils::{self, *};
use fiskeridir_rs::{Gear, GearGroup, LandingId};
use futures::TryStreamExt;

use crate::{
    error::ApiError,
    extractors::{BwPolicy, BwProfile},
    response::Response,
    *,
};
use actix_web::{
    web::{self, Path},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use kyogre_core::{
    Delivery, FiskeridirVesselId, HaulId, Ordering, Pagination, TripId, TripSorting, Trips,
    TripsQuery, VesselEventType,
};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use super::{
    fishing_facility::FishingFacility,
    haul::{HaulCatch, WhaleCatch},
};

#[derive(Default, Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct TripsParameters {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub ordering: Option<Ordering>,
    #[param(value_type = Option<String>, example = "RKAI,FKAI")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub delivery_points: Option<Vec<String>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub min_weight: Option<f64>,
    pub max_weight: Option<f64>,
    pub sorting: Option<TripSorting>,
    #[param(value_type = Option<String>, example = "2,5")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub gear_group_ids: Option<Vec<GearGroupId>>,
    #[param(value_type = Option<String>, example = "201,302")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub species_group_ids: Option<Vec<SpeciesGroupId>>,
    #[param(value_type = Option<String>, example = "1,3")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub vessel_length_groups: Option<Vec<utils::VesselLengthGroup>>,
    #[param(value_type = Option<String>, example = "2000013801,2001015304")]
    #[serde(deserialize_with = "deserialize_string_list", default)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
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
    profile: Option<BwProfile>,
    haul_id: Path<i64>,
) -> Result<Response<Option<Trip>>, ApiError> {
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    db.detailed_trip_of_haul(&HaulId(haul_id.into_inner()), read_fishing_facility)
        .await
        .map(|t| Response::new(t.map(Trip::from)))
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve trip of haul: {:?}", e);
            ApiError::InternalServerError
        })
}

#[utoipa::path(
    get,
    path = "/trip_of_landing/{landing_id}",
    responses(
        (status = 200, description = "trip associated with the given landing_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_of_landing<T: Database + 'static>(
    db: web::Data<T>,
    profile: Option<BwProfile>,
    landing_id: Path<String>,
) -> Result<Response<Option<Trip>>, ApiError> {
    let landing_id = landing_id.into_inner().try_into().map_err(|e| {
        event!(Level::ERROR, "invalid landing id: {:?}", e);
        ApiError::InvalidLandingId
    })?;
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    db.detailed_trip_of_landing(&landing_id, read_fishing_facility)
        .await
        .map(|t| Response::new(t.map(Trip::from)))
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve trip of landing: {:?}", e);
            ApiError::InternalServerError
        })
}

#[utoipa::path(
    get,
    path = "/trip_of_partial_landing/{landing_id}",
    responses(
        (status = 200, description = "trip associated with the given partial landing_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_of_partial_landing<T: Database + 'static>(
    db: web::Data<T>,
    profile: Option<BwProfile>,
    landing_id: Path<String>,
) -> Result<Response<Option<Trip>>, ApiError> {
    let landing_id = landing_id.into_inner();
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    db.detailed_trip_of_partial_landing(landing_id, read_fishing_facility)
        .await
        .map(|t| Response::new(t.map(Trip::from)))
        .map_err(|e| {
            event!(
                Level::ERROR,
                "failed to retrieve trip of partial landing: {:?}",
                e
            );
            ApiError::InternalServerError
        })
}

#[utoipa::path(
    get,
    path = "/trips",
    params(TripsParameters),
    responses(
        (status = 200, description = "trips matching the given parameters", body = [Trip]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trips<T: Database + 'static>(
    db: web::Data<T>,
    profile: Option<BwProfile>,
    params: web::Query<TripsParameters>,
) -> Result<HttpResponse, ApiError> {
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);
    let params = params.into_inner();

    match (params.start_date, params.end_date) {
        (Some(start), Some(end)) => {
            if start > end {
                let err = ApiError::StartAfterEnd { start, end };
                event!(Level::WARN, "{:?}", err);
                Err(err)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }?;

    let query = TripsQuery::from(params);

    to_streaming_response! {
        db.detailed_trips(
            query,
            read_fishing_facility,
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

#[utoipa::path(
    get,
    path = "/trips/current/{fiskeridir_vessel_id}",
    responses(
        (status = 200, description = "current trip of the given vessel", body = CurrentTrip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn current_trip<T: Database + 'static>(
    db: web::Data<T>,
    profile: Option<BwProfile>,
    fiskeridir_vessel_id: Path<u64>,
) -> Result<Response<Option<CurrentTrip>>, ApiError> {
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    Ok(Response::new(
        db.current_trip(
            FiskeridirVesselId(fiskeridir_vessel_id.into_inner() as i64),
            read_fishing_facility,
        )
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve current_trip: {:?}", e);
            ApiError::InternalServerError
        })?
        .map(CurrentTrip::from),
    ))
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
    pub landing_coverage_start: DateTime<Utc>,
    pub landing_coverage_end: DateTime<Utc>,
    pub num_deliveries: u32,
    pub most_recent_delivery_date: Option<DateTime<Utc>>,
    #[schema(value_type = Vec<u32>)]
    pub gear_ids: Vec<fiskeridir_rs::Gear>,
    #[schema(value_type = Vec<String>)]
    pub delivery_point_ids: Vec<fiskeridir_rs::DeliveryPointId>,
    pub hauls: Vec<TripHaul>,
    pub fishing_facilities: Vec<FishingFacility>,
    pub delivery: Delivery,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub events: Vec<VesselEvent>,
    pub trip_assembler_id: TripAssemblerId,
    #[schema(value_type = Vec<String>)]
    pub landing_ids: Vec<LandingId>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TripAssemblerId {
    Landings,
    Ers,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentTrip {
    pub departure: DateTime<Utc>,
    pub target_species_fiskeridir_id: Option<i32>,
    pub hauls: Vec<TripHaul>,
    pub fishing_facilities: Vec<FishingFacility>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VesselEvent {
    pub event_id: u64,
    #[schema(value_type = i64)]
    pub event_type: VesselEventType,
    pub event_name: String,
    pub report_timestamp: DateTime<Utc>,
    pub occurence_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TripHaul {
    #[schema(value_type = i64)]
    pub haul_id: HaulId,
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub stop_timestamp: DateTime<Utc>,
    pub total_living_weight: i64,
    #[schema(value_type = i32)]
    pub gear_id: Gear,
    #[schema(value_type = i32)]
    pub gear_group_id: GearGroup,
    pub fiskeridir_vessel_id: Option<i64>,
    pub catches: Vec<HaulCatch>,
    pub whale_catches: Vec<WhaleCatch>,
}

impl From<TripsParameters> for TripsQuery {
    fn from(value: TripsParameters) -> Self {
        TripsQuery {
            pagination: Pagination::<Trips>::new(value.limit, value.offset),
            ordering: value.ordering.unwrap_or_default(),
            sorting: value.sorting.unwrap_or_default(),
            delivery_points: value.delivery_points,
            start_date: value.start_date,
            end_date: value.end_date,
            min_weight: value.min_weight,
            max_weight: value.max_weight,
            gear_group_ids: value
                .gear_group_ids
                .map(|v| v.into_iter().map(|g| g.0).collect()),
            species_group_ids: value
                .species_group_ids
                .map(|v| v.into_iter().map(|g| g.0).collect()),
            vessel_length_groups: value
                .vessel_length_groups
                .map(|v| v.into_iter().map(|g| g.0).collect()),
            fiskeridir_vessel_ids: value.fiskeridir_vessel_ids,
        }
    }
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
            hauls: value.hauls.into_iter().map(TripHaul::from).collect(),
            fishing_facilities: value
                .fishing_facilities
                .into_iter()
                .map(FishingFacility::from)
                .collect(),
            delivery: value.delivery,
            start_port_id: value.start_port_id,
            end_port_id: value.end_port_id,
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            events: value
                .vessel_events
                .into_iter()
                .map(VesselEvent::from)
                .collect(),
            landing_ids: value.landing_ids,
            trip_assembler_id: value.assembler_id.into(),
            landing_coverage_start: value.landing_coverage.start(),
            landing_coverage_end: value.landing_coverage.end(),
        }
    }
}

impl From<kyogre_core::CurrentTrip> for CurrentTrip {
    fn from(v: kyogre_core::CurrentTrip) -> Self {
        Self {
            departure: v.departure,
            target_species_fiskeridir_id: v.target_species_fiskeridir_id,
            hauls: v.hauls.into_iter().map(TripHaul::from).collect(),
            fishing_facilities: v
                .fishing_facilities
                .into_iter()
                .map(FishingFacility::from)
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
            report_timestamp: value.report_timestamp,
            occurence_timestamp: value.occurence_timestamp,
        }
    }
}

impl From<kyogre_core::TripHaul> for TripHaul {
    fn from(v: kyogre_core::TripHaul) -> Self {
        Self {
            haul_id: v.haul_id,
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: v.stop_timestamp,
            total_living_weight: v.total_living_weight,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            catches: v.catches.into_iter().map(HaulCatch::from).collect(),
            whale_catches: v.whale_catches.into_iter().map(WhaleCatch::from).collect(),
        }
    }
}

impl From<kyogre_core::Haul> for TripHaul {
    fn from(v: kyogre_core::Haul) -> Self {
        Self {
            haul_id: v.haul_id,
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: v.stop_timestamp,
            total_living_weight: v.total_living_weight,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            catches: v.catches.into_iter().map(HaulCatch::from).collect(),
            whale_catches: v.whale_catches.into_iter().map(WhaleCatch::from).collect(),
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

impl PartialEq<TripHaul> for kyogre_core::Haul {
    fn eq(&self, other: &TripHaul) -> bool {
        let converted: TripHaul = self.clone().into();
        converted.eq(other)
    }
}

impl PartialEq<kyogre_core::Haul> for TripHaul {
    fn eq(&self, other: &kyogre_core::Haul) -> bool {
        other.eq(self)
    }
}

impl From<kyogre_core::TripAssemblerId> for TripAssemblerId {
    fn from(value: kyogre_core::TripAssemblerId) -> Self {
        match value {
            kyogre_core::TripAssemblerId::Landings => TripAssemblerId::Landings,
            kyogre_core::TripAssemblerId::Ers => TripAssemblerId::Ers,
        }
    }
}
