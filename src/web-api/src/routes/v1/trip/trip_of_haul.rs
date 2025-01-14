use crate::{error::Result, extractors::OptionBwProfile, response::Response, *};
use actix_web::web::{self, Path};
use kyogre_core::HaulId;
use oasgen::{oasgen, OaSchema};
use serde::Deserialize;
use tracing::error;
use v1::trip::Trip;

#[derive(Debug, Deserialize, OaSchema)]
pub struct TripOfHaulPath {
    pub haul_id: HaulId,
}

/// Returns the trip associated with the given haul.
#[oasgen(skip(db, meilisearch), tags("Trip"))]
#[tracing::instrument(skip(db, meilisearch))]
pub async fn trip_of_haul<T: Database + 'static, M: Meilisearch + 'static>(
    db: web::Data<T>,
    meilisearch: web::Data<Option<M>>,
    profile: OptionBwProfile,
    path: Path<TripOfHaulPath>,
) -> Result<Response<Option<Trip>>> {
    let read_fishing_facility = profile.read_fishing_facilities();

    if let Some(meilisearch) = meilisearch.as_ref() {
        match meilisearch
            .trip_of_haul(&path.haul_id, read_fishing_facility)
            .await
        {
            Ok(v) => return Ok(Response::new(v.map(Trip::from))),
            Err(e) => {
                error!("meilisearch cache returned error: {e:?}");
            }
        }
    }

    let trip = db
        .detailed_trip_of_haul(&path.haul_id, read_fishing_facility)
        .await?;

    Ok(Response::new(trip.map(Trip::from)))
}
