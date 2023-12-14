use crate::extractors::{Auth0Permission, Auth0Profile};
use fiskeridir_rs::{Gear, GearGroup, LandingId};
use futures::TryStreamExt;
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};

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
    FiskeridirVesselId, HaulId, Ordering, Pagination, TripAssemblerId, TripId, TripSorting, Trips,
    TripsQuery, VesselEventType,
};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use super::{
    fishing_facility::FishingFacility,
    haul::{HaulCatch, WhaleCatch},
};

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct TripsParameters {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub ordering: Option<Ordering>,
    #[param(rename = "deliveryPoints[]", value_type = Option<Vec<String>>)]
    pub delivery_points: Option<Vec<String>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub min_weight: Option<f64>,
    pub max_weight: Option<f64>,
    pub sorting: Option<TripSorting>,
    #[param(rename = "gearGroupIds[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub gear_group_ids: Option<Vec<GearGroup>>,
    #[param(rename = "speciesGroupIds[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    #[param(rename = "vesselLengthGroups[]", value_type = Option<Vec<String>>)]
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    #[param(rename = "fiskeridirVesselIds[]", value_type = Option<Vec<i64>>)]
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TripOfHaulPath {
    #[param(value_type = i64)]
    pub haul_id: HaulId,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TripOfLandingPath {
    #[param(value_type = String)]
    pub landing_id: LandingId,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TripOfPartialLandingPath {
    pub landing_id: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CurrentTripPath {
    #[param(value_type = i64)]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
}

#[utoipa::path(
    get,
    path = "/trip_of_haul/{haul_id}",
    params(TripOfHaulPath),
    responses(
        (status = 200, description = "trip associated with the given haul_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db, meilisearch))]
pub async fn trip_of_haul<T: Database + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    profile: Option<BwProfile>,
    path: Path<TripOfHaulPath>,
) -> Result<Response<Option<Trip>>, ApiError> {
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    if let Some(meilisearch) = meilisearch.as_ref() {
        match meilisearch
            .trip_of_haul(&path.haul_id, read_fishing_facility)
            .await
        {
            Ok(trip) => {
                return Ok(Response::new(trip.map(Trip::from)));
            }
            Err(e) => event!(
                Level::ERROR,
                "failed to retrieve trip_of_haul from meilisearch: {:?}",
                e
            ),
        }
    }

    db.detailed_trip_of_haul(&path.haul_id, read_fishing_facility)
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
    params(TripOfLandingPath),
    responses(
        (status = 200, description = "trip associated with the given landing_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db, meilisearch))]
pub async fn trip_of_landing<T: Database + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    profile: Option<BwProfile>,
    path: Path<TripOfLandingPath>,
) -> Result<Response<Option<Trip>>, ApiError> {
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    if let Some(meilisearch) = meilisearch.as_ref() {
        match meilisearch
            .trip_of_landing(&path.landing_id, read_fishing_facility)
            .await
        {
            Ok(trip) => {
                return Ok(Response::new(trip.map(Trip::from)));
            }
            Err(e) => event!(
                Level::ERROR,
                "failed to retrieve trip_of_landing from meilisearch: {:?}",
                e
            ),
        }
    }

    db.detailed_trip_of_landing(&path.landing_id, read_fishing_facility)
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
    params(TripOfPartialLandingPath),
    responses(
        (status = 200, description = "trip associated with the given partial landing_id", body = Trip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_of_partial_landing<T: Database + 'static>(
    db: web::Data<T>,
    bw_profile: Option<BwProfile>,
    auth: Option<Auth0Profile>,
    path: Path<TripOfPartialLandingPath>,
) -> Result<Response<Option<Trip>>, ApiError> {
    let read_fishing_facility = bw_profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false)
        || auth
            .map(|a| {
                a.permissions
                    .contains(&Auth0Permission::ReadFishingFacility)
            })
            .unwrap_or(false);

    db.detailed_trip_of_partial_landing(path.into_inner().landing_id, read_fishing_facility)
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
#[tracing::instrument(skip(db, meilisearch))]
pub async fn trips<T: Database + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    profile: Option<BwProfile>,
    params: Query<TripsParameters>,
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

    if let Some(meilisearch) = meilisearch.as_ref() {
        match meilisearch
            .trips(query.clone(), read_fishing_facility)
            .await
        {
            Ok(trips) => {
                let trips = trips.into_iter().map(Trip::from).collect::<Vec<_>>();
                return Ok(Response::new(trips).into());
            }
            Err(e) => event!(
                Level::ERROR,
                "failed to retrieve trips from meilisearch: {:?}",
                e
            ),
        }
    }

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
    params(CurrentTripPath),
    responses(
        (status = 200, description = "current trip of the given vessel", body = CurrentTrip),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn current_trip<T: Database + 'static>(
    db: web::Data<T>,
    profile: Option<BwProfile>,
    path: Path<CurrentTripPath>,
) -> Result<Response<Option<CurrentTrip>>, ApiError> {
    let read_fishing_facility = profile
        .map(|p| {
            p.policies
                .contains(&BwPolicy::BwReadExtendedFishingFacility)
        })
        .unwrap_or(false);

    Ok(Response::new(
        db.current_trip(path.fiskeridir_vessel_id, read_fishing_facility)
            .await
            .map_err(|e| {
                event!(Level::ERROR, "failed to retrieve current_trip: {:?}", e);
                ApiError::InternalServerError
            })?
            .map(CurrentTrip::from),
    ))
}

#[serde_as]
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
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub gear_ids: Vec<Gear>,
    #[schema(value_type = Vec<String>)]
    pub delivery_point_ids: Vec<fiskeridir_rs::DeliveryPointId>,
    pub hauls: Vec<TripHaul>,
    pub fishing_facilities: Vec<FishingFacility>,
    pub delivery: Delivery,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub events: Vec<VesselEvent>,
    #[serde_as(as = "DisplayFromStr")]
    pub trip_assembler_id: TripAssemblerId,
    #[schema(value_type = Vec<String>)]
    pub landing_ids: Vec<LandingId>,
    pub target_species_fiskeridir_id: Option<u32>,
    pub target_species_fao_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub delivered: Vec<Catch>,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Catch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    #[serde_as(as = "DisplayFromStr")]
    pub product_quality_id: Quality,
    pub product_quality_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentTrip {
    pub departure: DateTime<Utc>,
    pub target_species_fiskeridir_id: Option<i32>,
    pub hauls: Vec<TripHaul>,
    pub fishing_facilities: Vec<FishingFacility>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VesselEvent {
    pub event_id: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub event_type: VesselEventType,
    pub event_name: String,
    pub report_timestamp: DateTime<Utc>,
    pub occurence_timestamp: Option<DateTime<Utc>>,
}

#[serde_as]
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
    #[serde_as(as = "DisplayFromStr")]
    pub gear_id: Gear,
    #[serde_as(as = "DisplayFromStr")]
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
            gear_group_ids: value.gear_group_ids,
            species_group_ids: value.species_group_ids,
            vessel_length_groups: value.vessel_length_groups,
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
            delivery: value.delivery.into(),
            start_port_id: value.start_port_id,
            end_port_id: value.end_port_id,
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            events: value
                .vessel_events
                .into_iter()
                .map(VesselEvent::from)
                .collect(),
            landing_ids: value.landing_ids,
            trip_assembler_id: value.assembler_id,
            landing_coverage_start: value.landing_coverage.start(),
            landing_coverage_end: value.landing_coverage.end(),
            target_species_fiskeridir_id: value.target_species_fiskeridir_id,
            target_species_fao_id: value.target_species_fao_id,
        }
    }
}

impl From<kyogre_core::Delivery> for Delivery {
    fn from(v: kyogre_core::Delivery) -> Self {
        Self {
            delivered: v.delivered.into_iter().map(Catch::from).collect(),
            total_living_weight: v.total_living_weight,
            total_product_weight: v.total_product_weight,
            total_gross_weight: v.total_gross_weight,
        }
    }
}

impl From<kyogre_core::Catch> for Catch {
    fn from(v: kyogre_core::Catch) -> Self {
        Self {
            living_weight: v.living_weight,
            gross_weight: v.gross_weight,
            product_weight: v.product_weight,
            species_fiskeridir_id: v.species_fiskeridir_id,
            product_quality_id: v.product_quality_id,
            product_quality_name: v.product_quality_name,
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
