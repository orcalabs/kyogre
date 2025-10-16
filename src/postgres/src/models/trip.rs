use super::{TripTra, VesselEvent};
use crate::{
    error::{Error, Result},
    models::VesselEventDetailed,
    queries::{opt_type_to_i32, type_to_i32, type_to_i64},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    AisVmsPosition, Catch, DateRange, FishingFacility, FiskeridirVesselId, HasTrack, Haul,
    MinimalVesselEvent, PositionType, PrecisionId, PrecisionOutcome, ProcessingStatus,
    PrunedTripPosition, TripAssemblerConflict, TripAssemblerId, TripDistancerId, TripId,
    TripPositionLayerId, TripProcessingUnit, TripsConflictStrategy, VesselEventType,
};
use sqlx::postgres::types::PgRange;
use std::str::FromStr;
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: TripId,
    pub period: DateRange,
    pub period_extended: DateRange,
    pub period_precision: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub distance: Option<f64>,
    pub trip_assembler_id: TripAssemblerId,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub first_arrival: Option<DateTime<Utc>>,
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
    pub start_vessel_event_id: Option<i64>,
    pub end_vessel_event_id: i64,
    pub first_arrival: Option<DateTime<Utc>>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_assembler_id: TripAssemblerId,
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    #[unnest_insert(sql_type = "tstzrange")]
    pub landing_coverage: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period_extended: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub start_precision_id: Option<PrecisionId>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub end_precision_id: Option<PrecisionId>,
    pub start_precision_direction: Option<&'static str>,
    pub end_precision_direction: Option<&'static str>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_precision_status_id: ProcessingStatus,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub distance: Option<f64>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub distancer_id: Option<TripDistancerId>,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub position_layers_status: ProcessingStatus,
    pub track_coverage: f64,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_position_cargo_weight_distribution_status: ProcessingStatus,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_position_fuel_consumption_distribution_status: ProcessingStatus,
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
    pub trip_cumulative_cargo_weight: f64,
    pub trip_cumulative_fuel_consumption_liter: f64,
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

#[derive(Debug, Clone, PartialEq)]
pub struct TripAndSucceedingEventsLandings {
    pub fiskeridir_vessel_id: FiskeridirVesselId,

    pub start_vessel_event_id: i64,
    pub start_report_timestamp: DateTime<Utc>,

    pub end_vessel_event_id: i64,
    pub end_report_timestamp: DateTime<Utc>,

    pub landings_after_trip: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripAndSucceedingEventsErs {
    pub fiskeridir_vessel_id: FiskeridirVesselId,

    pub arrival_vessel_event_id: i64,
    pub arrival_port_id: Option<String>,
    pub arrival_report_timestamp: DateTime<Utc>,
    pub arrival_estimated_timestamp: DateTime<Utc>,

    pub departure_vessel_event_id: i64,
    pub departure_port_id: Option<String>,
    pub departure_report_timestamp: DateTime<Utc>,
    pub departure_estimated_timestamp: DateTime<Utc>,

    pub por_and_dep_events_after_trip: String,
}

impl TryFrom<TripAndSucceedingEventsLandings> for kyogre_core::TripAndSucceedingEvents {
    type Error = Error;

    fn try_from(value: TripAndSucceedingEventsLandings) -> std::result::Result<Self, Self::Error> {
        let TripAndSucceedingEventsLandings {
            fiskeridir_vessel_id,
            start_vessel_event_id,
            start_report_timestamp,
            end_vessel_event_id,
            end_report_timestamp,
            landings_after_trip,
        } = value;

        let start_event = kyogre_core::VesselEventDetailed {
            event_id: start_vessel_event_id as u64,
            vessel_id: fiskeridir_vessel_id,
            reported_timestamp: start_report_timestamp,
            event_type: VesselEventType::Landing,
            event_data: kyogre_core::VesselEventData::Landing,
        };

        let end_event = kyogre_core::VesselEventDetailed {
            event_id: end_vessel_event_id as u64,
            vessel_id: fiskeridir_vessel_id,
            reported_timestamp: end_report_timestamp,
            event_type: VesselEventType::Landing,
            event_data: kyogre_core::VesselEventData::Landing,
        };

        Ok(Self {
            start_and_end_event: [start_event, end_event],
            succeeding_events: serde_json::from_str::<Vec<VesselEventDetailed>>(
                &landings_after_trip,
            )?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<kyogre_core::VesselEventDetailed>>>()?,
        })
    }
}

impl TryFrom<TripAndSucceedingEventsErs> for kyogre_core::TripAndSucceedingEvents {
    type Error = Error;

    fn try_from(value: TripAndSucceedingEventsErs) -> std::result::Result<Self, Self::Error> {
        let TripAndSucceedingEventsErs {
            fiskeridir_vessel_id,
            arrival_vessel_event_id,
            arrival_port_id,
            arrival_report_timestamp,
            arrival_estimated_timestamp,
            departure_vessel_event_id,
            departure_port_id,
            departure_report_timestamp,
            departure_estimated_timestamp,
            por_and_dep_events_after_trip,
        } = value;

        let start_event = kyogre_core::VesselEventDetailed {
            event_id: departure_vessel_event_id as u64,
            vessel_id: fiskeridir_vessel_id,
            reported_timestamp: departure_report_timestamp,
            event_type: VesselEventType::ErsDep,
            event_data: kyogre_core::VesselEventData::ErsDep {
                port_id: departure_port_id,
                estimated_timestamp: departure_estimated_timestamp,
            },
        };

        let end_event = kyogre_core::VesselEventDetailed {
            event_id: arrival_vessel_event_id as u64,
            vessel_id: fiskeridir_vessel_id,
            reported_timestamp: arrival_report_timestamp,
            event_type: VesselEventType::ErsPor,
            event_data: kyogre_core::VesselEventData::ErsPor {
                port_id: arrival_port_id,
                estimated_timestamp: arrival_estimated_timestamp,
            },
        };

        Ok(Self {
            start_and_end_event: [start_event, end_event],
            succeeding_events: serde_json::from_str::<Vec<VesselEventDetailed>>(
                &por_and_dep_events_after_trip,
            )?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<kyogre_core::VesselEventDetailed>>>()?,
        })
    }
}

impl From<&TripProcessingUnit> for NewTrip {
    fn from(value: &TripProcessingUnit) -> Self {
        let TripProcessingUnit {
            vessel_id,
            trip:
                kyogre_core::NewTrip {
                    period,
                    period_extended,
                    landing_coverage,
                    first_arrival,
                    start_port_code: _,
                    end_port_code: _,
                    start_vessel_event_id,
                    end_vessel_event_id,
                },
            trip_id: _,
            trip_assembler_id,
            start_port,
            end_port,
            start_dock_points: _,
            end_dock_points: _,
            positions: _,
            precision_outcome,
            distance_output,
            position_layers_output,
        } = value;

        let (
            start_precision_id,
            start_precision_direction,
            end_precision_id,
            end_precision_direction,
            period_precision,
            trip_precision_status_id,
        ) = match precision_outcome {
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
                    ProcessingStatus::Successful,
                ),
                PrecisionOutcome::Failed => {
                    (None, None, None, None, None, ProcessingStatus::Attempted)
                }
            },
            None => (None, None, None, None, None, ProcessingStatus::Unprocessed),
        };

        let (distance, distancer_id) = match distance_output {
            Some(v) => (v.distance, Some(v.distancer_id)),
            None => (None, None),
        };

        NewTrip {
            // `start_vessel_event_id` is `None` only for the first ever landing event which
            // contains an artifical landing event, for all other cases it should be `Some`.
            // `end_vessel_event_id` should always be `Some`.
            start_vessel_event_id: *start_vessel_event_id,
            end_vessel_event_id: end_vessel_event_id.unwrap(),
            period: PgRange::from(period),
            period_extended: PgRange::from(period_extended),
            period_precision,
            landing_coverage: PgRange::from(landing_coverage),
            trip_assembler_id: *trip_assembler_id,
            fiskeridir_vessel_id: *vessel_id,
            start_precision_id,
            end_precision_id,
            start_precision_direction: start_precision_direction.map(|v| v.name()),
            end_precision_direction: end_precision_direction.map(|v| v.name()),
            trip_precision_status_id,
            distance,
            distancer_id,
            start_port_id: start_port.clone().map(|p| p.id),
            end_port_id: end_port.clone().map(|p| p.id),
            track_coverage: position_layers_output
                .as_ref()
                .map(|v| v.track_coverage)
                .unwrap_or(0.),
            position_layers_status: ProcessingStatus::Successful,
            trip_position_cargo_weight_distribution_status: ProcessingStatus::Successful,
            trip_position_fuel_consumption_distribution_status: ProcessingStatus::Successful,
            first_arrival: *first_arrival,
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
    pub period_extended: DateRange,
    pub period_precision: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub num_deliveries: i64,
    pub total_living_weight: f64,
    pub total_gross_weight: f64,
    pub total_product_weight: f64,
    pub total_price_for_fisher: f64,
    pub price_for_fisher_is_estimated: bool,
    pub delivery_points: Vec<DeliveryPointId>,
    pub gear_ids: Vec<Gear>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub landing_ids: Vec<LandingId>,
    pub latest_landing_timestamp: Option<DateTime<Utc>>,
    pub catches: String,
    pub hauls: String,
    pub tra: String,
    pub fishing_facilities: String,
    pub vessel_events: String,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub first_arrival: Option<DateTime<Utc>>,
    pub trip_assembler_id: TripAssemblerId,
    pub distance: Option<f64>,
    pub cache_version: i64,
    pub target_species_fiskeridir_id: Option<i32>,
    pub target_species_fao_id: Option<String>,
    pub fuel_consumption_liter: Option<f64>,
    pub track_coverage: f64,
    pub has_track: HasTrack,
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
        let Trip {
            trip_id,
            period,
            period_extended,
            period_precision,
            landing_coverage,
            distance,
            trip_assembler_id,
            start_port_id,
            end_port_id,
            target_species_fiskeridir_id,
            target_species_fao_id,
            first_arrival,
        } = value;

        Self {
            trip_id,
            period,
            period_extended,
            landing_coverage,
            distance,
            assembler_id: trip_assembler_id,
            period_precision,
            start_port_code: start_port_id,
            end_port_code: end_port_id,
            target_species_fiskeridir_id: target_species_fiskeridir_id.map(|v| v as u32),
            target_species_fao_id,
            first_arrival,
        }
    }
}

impl TryFrom<CurrentTrip> for kyogre_core::CurrentTrip {
    type Error = Error;

    fn try_from(v: CurrentTrip) -> Result<Self> {
        let CurrentTrip {
            departure_timestamp,
            target_species_fiskeridir_id,
            hauls,
            fishing_facilities,
        } = v;

        Ok(Self {
            departure: departure_timestamp,
            target_species_fiskeridir_id,
            hauls: serde_json::from_str::<Vec<Haul>>(&hauls)?,
            fishing_facilities: serde_json::from_str::<Vec<FishingFacility>>(&fishing_facilities)?,
        })
    }
}

impl From<TripCalculationTimer> for kyogre_core::TripCalculationTimer {
    fn from(v: TripCalculationTimer) -> Self {
        let TripCalculationTimer {
            timestamp,
            fiskeridir_vessel_id,
            queued_reset,
            conflict,
            conflict_vessel_event_id,
            conflict_event_type,
            conflict_vessel_event_timestamp,
        } = v;

        let conflict = match (
            conflict,
            conflict_event_type,
            conflict_vessel_event_timestamp,
        ) {
            (Some(timestamp), Some(event_type), Some(vessel_event_timestamp)) => {
                Some(TripAssemblerConflict {
                    timestamp,
                    event_type,
                    vessel_event_id: conflict_vessel_event_id,
                    vessel_event_timestamp,
                })
            }
            _ => None,
        };
        Self {
            fiskeridir_vessel_id,
            timestamp,
            queued_reset,
            conflict,
        }
    }
}

impl TryFrom<TripDetailed> for kyogre_core::TripDetailed {
    type Error = Error;

    fn try_from(value: TripDetailed) -> std::result::Result<Self, Self::Error> {
        let TripDetailed {
            trip_id,
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            period,
            period_extended,
            period_precision,
            landing_coverage,
            num_deliveries,
            total_living_weight,
            total_gross_weight,
            total_product_weight,
            total_price_for_fisher,
            price_for_fisher_is_estimated,
            delivery_points,
            gear_ids,
            gear_group_ids,
            species_group_ids,
            landing_ids,
            latest_landing_timestamp,
            catches,
            hauls,
            tra,
            fishing_facilities,
            vessel_events,
            start_port_id,
            end_port_id,
            trip_assembler_id,
            distance,
            cache_version,
            target_species_fiskeridir_id,
            target_species_fao_id,
            fuel_consumption_liter,
            track_coverage,
            has_track,
            first_arrival,
        } = value;

        Ok(kyogre_core::TripDetailed {
            period_precision,
            fiskeridir_vessel_id,
            fiskeridir_length_group_id,
            landing_coverage,
            trip_id,
            period,
            period_extended,
            num_deliveries: num_deliveries as u32,
            most_recent_delivery_date: latest_landing_timestamp,
            gear_ids,
            gear_group_ids,
            species_group_ids,
            delivery_point_ids: delivery_points,
            hauls: serde_json::from_str::<Vec<Haul>>(&hauls)?,
            fishing_facilities: serde_json::from_str::<Vec<FishingFacility>>(&fishing_facilities)?,
            delivery: kyogre_core::Delivery {
                delivered: serde_json::from_str::<Vec<Catch>>(&catches)?,
                total_living_weight,
                total_gross_weight,
                total_product_weight,
                total_price_for_fisher,
                price_for_fisher_is_estimated,
            },
            start_port_id,
            end_port_id,
            assembler_id: trip_assembler_id,
            vessel_events: serde_json::from_str::<Vec<VesselEvent>>(&vessel_events)?
                .into_iter()
                .map(kyogre_core::VesselEvent::from)
                .collect(),
            landing_ids,
            distance,
            cache_version,
            target_species_fiskeridir_id: target_species_fiskeridir_id.map(|v| v as u32),
            target_species_fao_id,
            fuel_consumption_liter,
            track_coverage,
            tra: serde_json::from_str::<Vec<TripTra>>(&tra)?
                .into_iter()
                .map(kyogre_core::Tra::from)
                .collect(),
            has_track,
            first_arrival,
        })
    }
}

impl TryFrom<TripAssemblerLogEntry> for kyogre_core::TripAssemblerLogEntry {
    type Error = Error;

    fn try_from(value: TripAssemblerLogEntry) -> std::result::Result<Self, Self::Error> {
        let TripAssemblerLogEntry {
            trip_assembler_log_id,
            fiskeridir_vessel_id,
            calculation_timer_prior,
            calculation_timer_post,
            conflict,
            conflict_vessel_event_timestamp,
            conflict_vessel_event_id,
            conflict_vessel_event_type_id,
            conflict_strategy,
            prior_trip_vessel_events,
            new_vessel_events,
        } = value;

        Ok(Self {
            trip_assembler_log_id: trip_assembler_log_id as u64,
            vessel_id: fiskeridir_vessel_id,
            calculation_timer_prior,
            calculation_timer_post,
            conflict,
            conflict_vessel_event_timestamp,
            conflict_vessel_event_id: conflict_vessel_event_id.map(|v| v as u64),
            conflict_vessel_event_type_id,
            prior_trip_vessel_events: serde_json::from_str(&prior_trip_vessel_events)?,
            new_vessel_events: serde_json::from_str(&new_vessel_events)?,
            conflict_strategy: TripsConflictStrategy::from_str(&conflict_strategy)?,
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
            trip_cumulative_cargo_weight: p.trip_cumulative_cargo_weight,
            trip_cumulative_fuel_consumption_liter: p.trip_cumulative_fuel_consumption_liter,
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
