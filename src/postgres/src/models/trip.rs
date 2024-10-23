use super::VesselEvent;
use crate::{
    error::{Error, Result},
    queries::{opt_type_to_i32, type_to_i32, type_to_i64},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    AisVmsPosition, Catch, DateRange, FishingFacility, FiskeridirVesselId, Haul,
    MinimalVesselEvent, PositionType, PrecisionId, PrecisionOutcome, PrecisionStatus,
    ProcessingStatus, PrunedTripPosition, TripAssemblerConflict, TripAssemblerId, TripDistancerId,
    TripId, TripPositionLayerId, TripProcessingUnit, TripsConflictStrategy, VesselEventType,
};
use sqlx::postgres::types::PgRange;
use std::str::FromStr;
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: TripId,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub distance: Option<f64>,
    pub trip_assembler_id: TripAssemblerId,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub target_species_fiskeridir_id: Option<i32>,
    pub target_species_fao_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerLogEntry {
    pub trip_assembler_log_id: i64,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub calculation_timer_prior: Option<DateTime<Utc>>,
    pub calculation_timer_post: DateTime<Utc>,
    pub conflict: Option<DateTime<Utc>>,
    pub conflict_vessel_event_timestamp: Option<DateTime<Utc>>,
    pub conflict_vessel_event_id: Option<i64>,
    pub conflict_vessel_event_type_id: Option<VesselEventType>,
    pub conflict_strategy: String,
    pub prior_trip_vessel_events: String,
    pub new_vessel_events: String,
}

#[derive(Debug, Clone)]
pub struct NewTripAssemblerLogEntry<'a> {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub calculation_timer_prior_to_batch: Option<DateTime<Utc>>,
    pub calculation_timer_post_batch: DateTime<Utc>,
    pub conflict: Option<DateTime<Utc>>,
    pub conflict_vessel_event_timestamp: Option<DateTime<Utc>>,
    pub conflict_vessel_event_id: Option<i64>,
    pub conflict_vessel_event_type_id: Option<VesselEventType>,
    pub prior_trip_vessel_events: &'a [MinimalVesselEvent],
    pub conflict_strategy: TripsConflictStrategy,
    pub new_vessel_events: &'a [MinimalVesselEvent],
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "trips",
    returning = "trip_id:TripId, period, landing_coverage, fiskeridir_vessel_id"
)]
pub struct NewTrip {
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_assembler_id: TripAssemblerId,
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    #[unnest_insert(sql_type = "tstzrange")]
    pub landing_coverage: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub start_precision_id: Option<PrecisionId>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub end_precision_id: Option<PrecisionId>,
    pub start_precision_direction: Option<&'static str>,
    pub end_precision_direction: Option<&'static str>,
    pub trip_precision_status_id: &'static str,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub distance: Option<f64>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub distancer_id: Option<TripDistancerId>,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub position_layers_status: ProcessingStatus,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "trip_positions")]
pub struct TripAisVmsPosition {
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i64")]
    pub trip_id: TripId,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub speed: Option<f64>,
    pub navigation_status_id: Option<i32>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub position_type_id: PositionType,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub pruned_by: Option<TripPositionLayerId>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "trip_positions_pruned")]
pub struct TripPrunedAisVmsPosition {
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i64")]
    pub trip_id: TripId,
    #[unnest_insert(sql_type = "JSONB")]
    pub positions: serde_json::Value,
    #[unnest_insert(sql_type = "JSONB")]
    pub value: serde_json::Value,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_position_layer_id: TripPositionLayerId,
}

impl From<&TripProcessingUnit> for NewTrip {
    fn from(value: &TripProcessingUnit) -> Self {
        let (
            start_precision_id,
            start_precision_direction,
            end_precision_id,
            end_precision_direction,
            period_precision,
            trip_precision_status_id,
        ) = match &value.precision_outcome {
            Some(v) => match v {
                PrecisionOutcome::Success {
                    new_period,
                    start_precision,
                    end_precision,
                } => (
                    start_precision.as_ref().map(|v| v.id),
                    start_precision.as_ref().map(|v| v.direction),
                    end_precision.as_ref().map(|v| v.id),
                    end_precision.as_ref().map(|v| v.direction),
                    Some(PgRange::from(new_period)),
                    PrecisionStatus::Successful.name(),
                ),
                PrecisionOutcome::Failed => (
                    None,
                    None,
                    None,
                    None,
                    None,
                    PrecisionStatus::Attempted.name(),
                ),
            },
            None => (
                None,
                None,
                None,
                None,
                None,
                PrecisionStatus::Unprocessed.name(),
            ),
        };

        let (distance, distancer_id) = match value.distance_output {
            Some(v) => (v.distance, Some(v.distancer_id)),
            None => (None, None),
        };

        let position_layers_status = match value.trip_position_output {
            Some(_) => ProcessingStatus::Successful,
            None => ProcessingStatus::Unprocessed,
        };

        NewTrip {
            period: PgRange::from(&value.trip.period),
            period_precision,
            landing_coverage: PgRange::from(&value.trip.landing_coverage),
            trip_assembler_id: value.trip_assembler_id,
            fiskeridir_vessel_id: value.vessel_id,
            start_precision_id,
            end_precision_id,
            start_precision_direction: start_precision_direction.map(|v| v.name()),
            end_precision_direction: end_precision_direction.map(|v| v.name()),
            trip_precision_status_id,
            distance,
            distancer_id,
            start_port_id: value.start_port.clone().map(|p| p.id),
            end_port_id: value.end_port.clone().map(|p| p.id),
            position_layers_status,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurrentTrip {
    pub departure_timestamp: DateTime<Utc>,
    pub target_species_fiskeridir_id: Option<i32>,
    pub hauls: String,
    pub fishing_facilities: String,
}

#[derive(Debug, Clone)]
pub struct TripCalculationTimer {
    pub timestamp: DateTime<Utc>,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub queued_reset: bool,
    pub conflict: Option<DateTime<Utc>>,
    pub conflict_vessel_event_id: Option<i64>,
    pub conflict_event_type: Option<VesselEventType>,
    pub conflict_vessel_event_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct TripDetailed {
    pub trip_id: TripId,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub fiskeridir_length_group_id: VesselLengthGroup,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub num_deliveries: i64,
    pub total_living_weight: f64,
    pub total_gross_weight: f64,
    pub total_product_weight: f64,
    pub delivery_points: Vec<DeliveryPointId>,
    pub gear_ids: Vec<Gear>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub landing_ids: Vec<LandingId>,
    pub latest_landing_timestamp: Option<DateTime<Utc>>,
    pub catches: String,
    pub hauls: String,
    pub fishing_facilities: String,
    pub vessel_events: String,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub trip_assembler_id: TripAssemblerId,
    pub distance: Option<f64>,
    pub cache_version: i64,
    pub target_species_fiskeridir_id: Option<i32>,
    pub target_species_fao_id: Option<String>,
    pub fuel_consumption: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct NewTripAssemblerConflict {
    pub timestamp: DateTime<Utc>,
    pub vessel_event_timestamp: DateTime<Utc>,
    pub vessel_event_id: Option<i64>,
    pub event_type: VesselEventType,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
}

impl From<Trip> for kyogre_core::Trip {
    fn from(value: Trip) -> Self {
        Self {
            trip_id: value.trip_id,
            period: value.period,
            landing_coverage: value.landing_coverage,
            distance: value.distance,
            assembler_id: value.trip_assembler_id,
            precision_period: value.period_precision,
            start_port_code: value.start_port_id,
            end_port_code: value.end_port_id,
            target_species_fiskeridir_id: value.target_species_fiskeridir_id.map(|v| v as u32),
            target_species_fao_id: value.target_species_fao_id,
        }
    }
}

impl TryFrom<CurrentTrip> for kyogre_core::CurrentTrip {
    type Error = Error;

    fn try_from(v: CurrentTrip) -> Result<Self> {
        Ok(Self {
            departure: v.departure_timestamp,
            target_species_fiskeridir_id: v.target_species_fiskeridir_id,
            hauls: serde_json::from_str::<Vec<Haul>>(&v.hauls)?,
            fishing_facilities: serde_json::from_str::<Vec<FishingFacility>>(
                &v.fishing_facilities,
            )?,
        })
    }
}

impl From<TripCalculationTimer> for kyogre_core::TripCalculationTimer {
    fn from(v: TripCalculationTimer) -> Self {
        let conflict = match (
            v.conflict,
            v.conflict_event_type,
            v.conflict_vessel_event_timestamp,
        ) {
            (Some(timestamp), Some(event_type), Some(vessel_event_timestamp)) => {
                Some(TripAssemblerConflict {
                    timestamp,
                    event_type,
                    vessel_event_id: v.conflict_vessel_event_id,
                    vessel_event_timestamp,
                })
            }
            _ => None,
        };
        Self {
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            timestamp: v.timestamp,
            queued_reset: v.queued_reset,
            conflict,
        }
    }
}
impl TryFrom<TripDetailed> for kyogre_core::TripDetailed {
    type Error = Error;

    fn try_from(value: TripDetailed) -> std::result::Result<Self, Self::Error> {
        Ok(kyogre_core::TripDetailed {
            period_precision: value.period_precision,
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            fiskeridir_length_group_id: value.fiskeridir_length_group_id,
            landing_coverage: value.landing_coverage,
            trip_id: value.trip_id,
            period: value.period,
            num_deliveries: value.num_deliveries as u32,
            most_recent_delivery_date: value.latest_landing_timestamp,
            gear_ids: value.gear_ids,
            gear_group_ids: value.gear_group_ids,
            species_group_ids: value.species_group_ids,
            delivery_point_ids: value.delivery_points,
            hauls: serde_json::from_str::<Vec<Haul>>(&value.hauls)?,
            fishing_facilities: serde_json::from_str::<Vec<FishingFacility>>(
                &value.fishing_facilities,
            )?,
            delivery: kyogre_core::Delivery {
                delivered: serde_json::from_str::<Vec<Catch>>(&value.catches)?,
                total_living_weight: value.total_living_weight,
                total_gross_weight: value.total_gross_weight,
                total_product_weight: value.total_product_weight,
            },
            start_port_id: value.start_port_id,
            end_port_id: value.end_port_id,
            assembler_id: value.trip_assembler_id,
            vessel_events: serde_json::from_str::<Vec<VesselEvent>>(&value.vessel_events)?
                .into_iter()
                .map(kyogre_core::VesselEvent::from)
                .collect(),
            landing_ids: value.landing_ids,
            distance: value.distance,
            cache_version: value.cache_version,
            target_species_fiskeridir_id: value.target_species_fiskeridir_id.map(|v| v as u32),
            target_species_fao_id: value.target_species_fao_id,
            fuel_consumption: value.fuel_consumption,
        })
    }
}

impl TryFrom<TripAssemblerLogEntry> for kyogre_core::TripAssemblerLogEntry {
    type Error = Error;

    fn try_from(value: TripAssemblerLogEntry) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            trip_assembler_log_id: value.trip_assembler_log_id as u64,
            vessel_id: value.fiskeridir_vessel_id,
            calculation_timer_prior: value.calculation_timer_prior,
            calculation_timer_post: value.calculation_timer_post,
            conflict: value.conflict,
            conflict_vessel_event_timestamp: value.conflict_vessel_event_timestamp,
            conflict_vessel_event_id: value.conflict_vessel_event_id.map(|v| v as u64),
            conflict_vessel_event_type_id: value.conflict_vessel_event_type_id,
            prior_trip_vessel_events: serde_json::from_str(&value.prior_trip_vessel_events)?,
            new_vessel_events: serde_json::from_str(&value.new_vessel_events)?,
            conflict_strategy: TripsConflictStrategy::from_str(&value.conflict_strategy)?,
        })
    }
}

impl TripAisVmsPosition {
    pub fn new(id: TripId, p: AisVmsPosition) -> Self {
        Self {
            trip_id: id,
            latitude: p.latitude,
            longitude: p.longitude,
            timestamp: p.timestamp,
            course_over_ground: p.course_over_ground,
            speed: p.speed,
            navigation_status_id: p.navigational_status.map(|v| v as i32),
            rate_of_turn: p.rate_of_turn,
            true_heading: p.true_heading,
            distance_to_shore: p.distance_to_shore,
            position_type_id: p.position_type,
            pruned_by: p.pruned_by,
        }
    }
}

impl TripPrunedAisVmsPosition {
    pub fn new(id: TripId, p: PrunedTripPosition) -> Self {
        Self {
            trip_id: id,
            positions: p.positions,
            value: p.value,
            trip_position_layer_id: p.trip_layer,
        }
    }
}
