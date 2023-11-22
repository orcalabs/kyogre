use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use error_stack::{report, Report, Result, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    DateRange, Delivery, FishingFacility, FiskeridirVesselId, HaulId, MeilisearchSource,
    TripAssemblerId, TripDetailed, TripHaul, TripId, VesselEvent,
};
use meilisearch_sdk::{Index, PaginationSetting, Settings};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{error::MeilisearchError, to_nanos, Id, IdVersion, Indexable, MeilisearchAdapter};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Trip {
    pub trip_id: TripId,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub fiskeridir_length_group_id: VesselLengthGroup,
    pub start: i64,
    pub end: i64,
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
    pub hauls: Vec<TripHaul>,
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
}

impl Trip {
    pub async fn create_index<T>(adapter: &MeilisearchAdapter<T>) -> Result<(), MeilisearchError> {
        let settings = Settings::new()
            .with_searchable_attributes(Vec::<String>::new())
            .with_ranking_rules(["sort"])
            .with_filterable_attributes([
                "fiskeridir_vessel_id",
                "fiskeridir_length_group_id",
                "start",
                "end",
                "total_living_weight",
                "gear_group_ids",
                "species_group_ids",
                "delivery_point_ids",
                "landing_ids",
                "haul_ids",
            ])
            .with_sortable_attributes(["end", "total_living_weight"])
            .with_pagination(PaginationSetting {
                max_total_hits: usize::MAX,
            });

        let task = Self::index(adapter)
            .set_settings(&settings)
            .await
            .change_context(MeilisearchError::Index)?
            .wait_for_completion(&adapter.client, None, Some(Duration::from_secs(60 * 10)))
            .await
            .change_context(MeilisearchError::Index)?;

        if !task.is_success() {
            return Err(report!(MeilisearchError::Index)
                .attach_printable(format!("create index did not succeed: {task:?}")));
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub(crate) struct TripIdVersion {
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

    fn index<T>(adapter: &MeilisearchAdapter<T>) -> Index {
        adapter.trips_index()
    }
    fn primary_key() -> &'static str {
        "trip_id"
    }
    async fn refresh<T: MeilisearchSource>(
        adapter: &MeilisearchAdapter<T>,
    ) -> Result<(), MeilisearchError> {
        let index = Self::index(adapter);

        let cache_versions = Self::all_versions(&index).await?;

        let source_versions = adapter
            .source
            .all_trip_versions()
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let (to_insert, to_delete) = Self::to_insert_and_delete(cache_versions, source_versions);

        event!(Level::INFO, "trips to delete: {}", to_delete.len());
        event!(Level::INFO, "trips to insert: {}", to_insert.len());

        let mut tasks = Vec::new();

        if let Some(task) = Self::delete_items(&index, &to_delete).await? {
            tasks.push(task);
        }

        for ids in to_insert.chunks(20_000) {
            let trips = adapter
                .source
                .trips_by_ids(ids)
                .await
                .change_context(MeilisearchError::Source)?
                .into_iter()
                .map(Trip::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            Self::add_items(&index, &mut tasks, &trips).await;
        }

        Self::wait_for_completion(&adapter.client, tasks).await?;

        Ok(())
    }
}

impl Trip {
    pub fn try_to_trip_detailed(
        self,
        read_fishing_facility: bool,
    ) -> Result<TripDetailed, MeilisearchError> {
        let start = Utc.timestamp_nanos(self.start);
        let end = Utc.timestamp_nanos(self.end);

        let period_precision = match (self.period_precision_start, self.period_precision_end) {
            (Some(start), Some(end)) => {
                Some(DateRange::new(start, end).change_context(MeilisearchError::DataConversion)?)
            }
            (None, None) => None,
            _ => return Err(report!(MeilisearchError::DataConversion)),
        };

        Ok(TripDetailed {
            trip_id: self.trip_id,
            fiskeridir_vessel_id: self.fiskeridir_vessel_id,
            fiskeridir_length_group_id: self.fiskeridir_length_group_id,
            period: DateRange::new(start, end).change_context(MeilisearchError::DataConversion)?,
            period_precision,
            landing_coverage: DateRange::new(
                self.landing_coverage_start,
                self.landing_coverage_end,
            )
            .change_context(MeilisearchError::DataConversion)?,
            num_deliveries: self.num_deliveries,
            most_recent_delivery_date: self.most_recent_delivery_date,
            gear_ids: self.gear_ids,
            gear_group_ids: self.gear_group_ids,
            species_group_ids: self.species_group_ids,
            delivery_point_ids: self.delivery_point_ids,
            hauls: self.hauls,
            fishing_facilities: if read_fishing_facility {
                self.fishing_facilities
            } else {
                vec![]
            },
            delivery: self.delivery,
            start_port_id: self.start_port_id,
            end_port_id: self.end_port_id,
            assembler_id: self.assembler_id,
            vessel_events: self.vessel_events,
            landing_ids: self.landing_ids,
            distance: self.distance,
            cache_version: self.cache_version,
        })
    }
}

impl TryFrom<TripDetailed> for Trip {
    type Error = Report<MeilisearchError>;

    fn try_from(v: TripDetailed) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            trip_id: v.trip_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            fiskeridir_length_group_id: v.fiskeridir_length_group_id,
            start: to_nanos(v.period.start())?,
            end: to_nanos(v.period.end())?,
            period_precision_start: v.period_precision.as_ref().map(|p| p.start()),
            period_precision_end: v.period_precision.map(|p| p.end()),
            landing_coverage_start: v.landing_coverage.start(),
            landing_coverage_end: v.landing_coverage.end(),
            num_deliveries: v.num_deliveries,
            most_recent_delivery_date: v.most_recent_delivery_date,
            gear_ids: v.gear_ids,
            gear_group_ids: v.gear_group_ids,
            species_group_ids: v.species_group_ids,
            delivery_point_ids: v.delivery_point_ids,
            haul_ids: v.hauls.iter().map(|h| h.haul_id).collect(),
            hauls: v.hauls,
            fishing_facilities: v.fishing_facilities,
            total_living_weight: v.delivery.total_living_weight,
            delivery: v.delivery,
            start_port_id: v.start_port_id,
            end_port_id: v.end_port_id,
            assembler_id: v.assembler_id,
            vessel_events: v.vessel_events,
            landing_ids: v.landing_ids,
            distance: v.distance,
            cache_version: v.cache_version,
        })
    }
}
