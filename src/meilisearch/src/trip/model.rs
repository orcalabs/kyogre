use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use error_stack::{report, Report, Result, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{DateRange, FiskeridirVesselId, TripAssemblerId, TripDetailed, TripId};
use meilisearch_sdk::{Client, PaginationSetting, Settings};
use serde::{Deserialize, Serialize};

use crate::error::MeilisearchError;

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
    pub hauls: String,
    pub fishing_facilities: String,
    pub delivery: String,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub assembler_id: TripAssemblerId,
    pub vessel_events: String,
    pub landing_ids: Vec<LandingId>,
    pub distance: Option<f64>,
    pub cache_version: i64,
    pub total_living_weight: f64,
}

impl Trip {
    pub async fn create_index(client: &Client) -> Result<(), MeilisearchError> {
        let settings = Settings::new()
            .with_filterable_attributes([
                "trip_id",
                "fiskeridir_vessel_id",
                "fiskeridir_length_group_id",
                "start",
                "end",
                "total_living_weight",
                "gear_group_ids",
                "species_group_ids",
                "delivery_point_ids",
            ])
            .with_sortable_attributes(["end", "total_living_weight"])
            .with_pagination(PaginationSetting {
                max_total_hits: usize::MAX,
            });

        let task = client
            .index(Self::index_name())
            .set_settings(&settings)
            .await
            .change_context(MeilisearchError::Index)?
            .wait_for_completion(client, None, Some(Duration::from_secs(60 * 10)))
            .await
            .change_context(MeilisearchError::Index)?;

        if !task.is_success() {
            return Err(report!(MeilisearchError::Index)
                .attach_printable(format!("create index did not succeed: {task:?}")));
        }

        Ok(())
    }

    pub const fn index_name() -> &'static str {
        "trips"
    }
    pub const fn primary_key() -> &'static str {
        "trip_id"
    }

    pub fn try_to_trip_detailed(
        self,
        read_fishing_facility: bool,
    ) -> Result<TripDetailed, MeilisearchError> {
        let secs = self.start / 1000;
        let nsecs = (self.start % 1000) * 1_000_000;
        let start = Utc.timestamp_opt(secs, nsecs as u32).unwrap();

        let secs = self.end / 1000;
        let nsecs = (self.end % 1000) * 1_000_000;
        let end = Utc.timestamp_opt(secs, nsecs as u32).unwrap();

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
            hauls: serde_json::from_str(&self.hauls)
                .change_context(MeilisearchError::DataConversion)?,
            fishing_facilities: if read_fishing_facility {
                serde_json::from_str(&self.fishing_facilities)
                    .change_context(MeilisearchError::DataConversion)?
            } else {
                vec![]
            },
            delivery: serde_json::from_str(&self.delivery)
                .change_context(MeilisearchError::DataConversion)?,
            start_port_id: self.start_port_id,
            end_port_id: self.end_port_id,
            assembler_id: self.assembler_id,
            vessel_events: serde_json::from_str(&self.vessel_events)
                .change_context(MeilisearchError::DataConversion)?,
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
            start: v.period.start().timestamp_millis(),
            end: v.period.end().timestamp_millis(),
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
            hauls: serde_json::to_string(&v.hauls)
                .change_context(MeilisearchError::DataConversion)?,
            fishing_facilities: serde_json::to_string(&v.fishing_facilities)
                .change_context(MeilisearchError::DataConversion)?,
            delivery: serde_json::to_string(&v.delivery)
                .change_context(MeilisearchError::DataConversion)?,
            start_port_id: v.start_port_id,
            end_port_id: v.end_port_id,
            assembler_id: v.assembler_id,
            vessel_events: serde_json::to_string(&v.vessel_events)
                .change_context(MeilisearchError::DataConversion)?,
            landing_ids: v.landing_ids,
            distance: v.distance,
            cache_version: v.cache_version,
            total_living_weight: v.delivery.total_living_weight,
        })
    }
}
