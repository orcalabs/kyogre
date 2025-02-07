use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    DateRange, Delivery, FishingFacility, FiskeridirVesselId, HasTrack, Haul, HaulId,
    MeilisearchSource, Tra, TripAssemblerId, TripDetailed, TripId, VesselEvent,
};
use serde::{Deserialize, Serialize};

use super::filter::{TripFilterDiscriminants, TripSort};
use crate::{
    error::{Error, Result},
    indexable::{Id, IdVersion, Indexable},
    utils::to_nanos,
    CacheIndex,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Trip {
    pub trip_id: TripId,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub fiskeridir_length_group_id: VesselLengthGroup,
    pub start: i64,
    pub end: i64,
    pub start_extended: i64,
    pub end_extended: i64,
    pub period_precision_start: Option<DateTime<Utc>>,
    pub period_precision_end: Option<DateTime<Utc>>,
    pub landing_coverage_start: DateTime<Utc>,
    pub landing_coverage_end: DateTime<Utc>,
    pub num_deliveries: u32,
    pub most_recent_delivery_date: Option<DateTime<Utc>>,
    pub gear_ids: Vec<Gear>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub delivery_point_ids: Vec<DeliveryPointId>,
    pub hauls: Vec<Haul>,
    pub tra: Vec<Tra>,
    pub fishing_facilities: Vec<FishingFacility>,
    pub delivery: Delivery,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub assembler_id: TripAssemblerId,
    pub vessel_events: Vec<VesselEvent>,
    pub landing_ids: Vec<LandingId>,
    pub haul_ids: Vec<HaulId>,
    pub distance: Option<f64>,
    pub cache_version: i64,
    pub total_living_weight: f64,
    pub target_species_fiskeridir_id: Option<u32>,
    pub target_species_fao: Option<String>,
    pub fuel_consumption: Option<f64>,
    pub track_coverage: Option<f64>,
    pub has_track: HasTrack,
}

#[derive(Deserialize)]
pub struct TripIdVersion {
    trip_id: TripId,
    cache_version: i64,
}

impl IdVersion for TripIdVersion {
    type Id = TripId;

    fn id(&self) -> Self::Id {
        self.trip_id
    }
    fn version(&self) -> i64 {
        self.cache_version
    }
}

impl Id for Trip {
    type Id = TripId;

    fn id(&self) -> Self::Id {
        self.trip_id
    }
}

#[async_trait]
impl Indexable for Trip {
    type Id = TripId;
    type Item = Trip;
    type IdVersion = TripIdVersion;
    type FilterableAttributes = TripFilterDiscriminants;
    type SortableAttributes = TripSort;

    fn cache_index() -> CacheIndex {
        CacheIndex::Trips
    }
    fn primary_key() -> &'static str {
        "trip_id"
    }
    fn chunk_size() -> usize {
        20_000
    }
    async fn source_versions<T: MeilisearchSource>(source: &T) -> Result<Vec<(Self::Id, i64)>> {
        Ok(source.all_trip_versions().await?)
    }
    async fn items_by_ids<T: MeilisearchSource>(
        source: &T,
        ids: &[Self::Id],
    ) -> Result<Vec<Self::Item>> {
        Ok(source
            .trips_by_ids(ids)
            .await?
            .into_iter()
            .map(Trip::try_from)
            .collect::<Result<Vec<_>>>()?)
    }
}

impl Trip {
    pub fn try_to_trip_detailed(self, read_fishing_facility: bool) -> Result<TripDetailed> {
        let Self {
            trip_id,
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            start,
            end,
            start_extended,
            end_extended,
            period_precision_start,
            period_precision_end,
            landing_coverage_start,
            landing_coverage_end,
            num_deliveries,
            most_recent_delivery_date,
            gear_ids,
            gear_group_ids,
            species_group_ids,
            delivery_point_ids,
            hauls,
            tra,
            fishing_facilities,
            delivery,
            start_port_id,
            end_port_id,
            assembler_id,
            vessel_events,
            landing_ids,
            haul_ids: _,
            distance,
            cache_version,
            total_living_weight: _,
            target_species_fiskeridir_id,
            target_species_fao,
            fuel_consumption,
            track_coverage,
            has_track,
        } = self;

        let start = Utc.timestamp_nanos(start);
        let end = Utc.timestamp_nanos(end);

        let start_extended = Utc.timestamp_nanos(start_extended);
        let end_extended = Utc.timestamp_nanos(end_extended);

        let period_precision = match (period_precision_start, period_precision_end) {
            (Some(start), Some(end)) => Some(DateRange::new(start, end)?),
            (None, None) => None,
            _ => unreachable!(),
        };

        Ok(TripDetailed {
            trip_id,
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            period: DateRange::new(start, end)?,
            period_extended: DateRange::new(start_extended, end_extended)?,
            period_precision,
            landing_coverage: DateRange::new(landing_coverage_start, landing_coverage_end)?,
            num_deliveries,
            most_recent_delivery_date,
            gear_ids,
            gear_group_ids,
            species_group_ids,
            delivery_point_ids,
            hauls,
            fishing_facilities: if read_fishing_facility {
                fishing_facilities
            } else {
                vec![]
            },
            delivery,
            start_port_id,
            end_port_id,
            assembler_id,
            vessel_events,
            landing_ids,
            distance,
            cache_version,
            target_species_fiskeridir_id,
            target_species_fao_id: target_species_fao,
            fuel_consumption,
            track_coverage,
            tra,
            has_track,
        })
    }
}

impl TryFrom<TripDetailed> for Trip {
    type Error = Error;

    fn try_from(v: TripDetailed) -> std::result::Result<Self, Self::Error> {
        let TripDetailed {
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            trip_id,
            period,
            period_extended,
            period_precision,
            landing_coverage,
            num_deliveries,
            most_recent_delivery_date,
            gear_ids,
            gear_group_ids,
            species_group_ids,
            delivery_point_ids,
            hauls,
            tra,
            fishing_facilities,
            delivery,
            start_port_id,
            end_port_id,
            assembler_id,
            vessel_events,
            landing_ids,
            distance,
            cache_version,
            target_species_fiskeridir_id,
            target_species_fao_id,
            fuel_consumption,
            track_coverage,
            has_track,
        } = v;

        Ok(Self {
            trip_id,
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            start: to_nanos(period.start())?,
            end: to_nanos(period.end())?,
            start_extended: to_nanos(period_extended.start())?,
            end_extended: to_nanos(period_extended.end())?,
            period_precision_start: period_precision.as_ref().map(|p| p.start()),
            period_precision_end: period_precision.map(|p| p.end()),
            landing_coverage_start: landing_coverage.start(),
            landing_coverage_end: landing_coverage.end(),
            num_deliveries,
            most_recent_delivery_date,
            gear_ids,
            gear_group_ids,
            species_group_ids,
            delivery_point_ids,
            haul_ids: hauls.iter().map(|h| h.id).collect(),
            hauls,
            fishing_facilities,
            total_living_weight: delivery.total_living_weight,
            delivery,
            start_port_id,
            end_port_id,
            assembler_id,
            vessel_events,
            landing_ids,
            distance,
            cache_version,
            target_species_fiskeridir_id,
            target_species_fao: target_species_fao_id,
            fuel_consumption,
            track_coverage,
            tra,
            has_track,
        })
    }
}
