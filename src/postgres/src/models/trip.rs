use super::{FishingFacility, HaulCatch, WhaleCatch};
use crate::queries::{enum_to_i32, float_to_decimal, opt_enum_to_i32};
use crate::{error::PostgresError, queries::decimal_to_float};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use fiskeridir_rs::{
    DeliveryPointId, Gear, GearGroup, LandingId, Quality, SpeciesGroup, VesselLengthGroup,
};
use kyogre_core::{
    DateRange, FiskeridirVesselId, HaulId, PositionType, PrecisionId, PrecisionOutcome,
    PrecisionStatus, ProcessingStatus, TripAssemblerId, TripDistancerId, TripId,
    TripPositionLayerId, TripProcessingUnit, VesselEventType,
};
use serde::Deserialize;
use sqlx::postgres::types::PgRange;
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: i64,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub distance: Option<BigDecimal>,
    pub trip_assembler_id: TripAssemblerId,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "trips",
    returning = "trip_id::bigint!, period, landing_coverage, fiskeridir_vessel_id"
)]
pub struct NewTrip {
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub trip_assembler_id: TripAssemblerId,
    pub fiskeridir_vessel_id: i64,
    #[unnest_insert(sql_type = "tstzrange")]
    pub landing_coverage: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period: PgRange<DateTime<Utc>>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_enum_to_i32")]
    pub start_precision_id: Option<PrecisionId>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_enum_to_i32")]
    pub end_precision_id: Option<PrecisionId>,
    pub start_precision_direction: Option<String>,
    pub end_precision_direction: Option<String>,
    pub trip_precision_status_id: String,
    #[unnest_insert(sql_type = "tstzrange")]
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub distance: Option<BigDecimal>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_enum_to_i32")]
    pub distancer_id: Option<TripDistancerId>,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub position_layers_status: ProcessingStatus,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "trip_positions")]
pub struct TripAisVmsPosition {
    pub trip_id: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub speed: Option<f64>,
    pub navigation_status_id: Option<i32>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub position_type_id: PositionType,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_enum_to_i32")]
    pub pruned_by: Option<TripPositionLayerId>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "trip_positions_pruned")]
pub struct TripPrunedAisVmsPosition {
    pub trip_id: i64,
    #[unnest_insert(sql_type = "JSONB")]
    pub positions: serde_json::Value,
    #[unnest_insert(sql_type = "JSONB")]
    pub value: serde_json::Value,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub trip_position_layer_id: TripPositionLayerId,
}

impl TryFrom<&TripProcessingUnit> for NewTrip {
    type Error = Report<PostgresError>;

    fn try_from(value: &TripProcessingUnit) -> Result<Self, Self::Error> {
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
            Some(v) => (
                match v
                    .distance
                    .map(float_to_decimal)
                    .transpose()
                    .change_context(PostgresError::DataConversion)
                {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                },
                Some(v.distancer_id),
            ),
            None => (None, None),
        };

        let position_layers_status = match value.trip_position_output {
            Some(_) => ProcessingStatus::Successful,
            None => ProcessingStatus::Unprocessed,
        };

        Ok(NewTrip {
            period: PgRange::from(&value.trip.period),
            period_precision,
            landing_coverage: PgRange::from(&value.trip.landing_coverage),
            trip_assembler_id: value.trip_assembler_id,
            fiskeridir_vessel_id: value.vessel_id.0,
            start_precision_id,
            end_precision_id,
            start_precision_direction: start_precision_direction.map(|v| v.name().to_string()),
            end_precision_direction: end_precision_direction.map(|v| v.name().to_string()),
            trip_precision_status_id: trip_precision_status_id.to_string(),
            distance,
            distancer_id,
            start_port_id: value.start_port.clone().map(|p| p.id),
            end_port_id: value.end_port.clone().map(|p| p.id),
            position_layers_status,
        })
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
    pub fiskeridir_vessel_id: i64,
    pub queued_reset: bool,
    pub conflict: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerConflict {
    pub fiskeridir_vessel_id: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub struct TripDetailed {
    pub trip_id: i64,
    pub fiskeridir_vessel_id: i64,
    pub fiskeridir_length_group_id: VesselLengthGroup,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub num_deliveries: i64,
    pub total_living_weight: BigDecimal,
    pub total_gross_weight: BigDecimal,
    pub total_product_weight: BigDecimal,
    pub delivery_points: Vec<String>,
    pub gear_ids: Vec<Gear>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub landing_ids: Vec<String>,
    pub latest_landing_timestamp: Option<DateTime<Utc>>,
    pub catches: String,
    pub hauls: String,
    pub fishing_facilities: String,
    pub vessel_events: String,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub trip_assembler_id: TripAssemblerId,
    pub distance: Option<BigDecimal>,
    pub cache_version: i64,
}

#[derive(Deserialize)]
struct TripHaul {
    haul_id: i64,
    ers_activity_id: String,
    duration: i32,
    haul_distance: Option<i32>,
    start_timestamp: DateTime<Utc>,
    stop_timestamp: DateTime<Utc>,
    start_latitude: BigDecimal,
    start_longitude: BigDecimal,
    stop_latitude: BigDecimal,
    stop_longitude: BigDecimal,
    total_living_weight: i64,
    gear_id: Gear,
    gear_group_id: GearGroup,
    fiskeridir_vessel_id: Option<i64>,
    catches: Vec<HaulCatch>,
    whale_catches: Vec<WhaleCatch>,
}

#[derive(Debug, Clone, Deserialize)]
struct Delivery {
    total_living_weight: f64,
    total_product_weight: f64,
    total_gross_weight: f64,
    catches: Vec<Catch>,
}

#[derive(Debug, Clone, Deserialize)]
struct Catch {
    living_weight: f64,
    gross_weight: f64,
    product_weight: f64,
    species_fiskeridir_id: i32,
    product_quality_id: Quality,
}

#[derive(Debug, Clone, Deserialize)]
struct VesselEvent {
    vessel_event_id: i64,
    fiskeridir_vessel_id: i64,
    report_timestamp: DateTime<Utc>,
    occurence_timestamp: Option<DateTime<Utc>>,
    vessel_event_type_id: VesselEventType,
}

#[derive(Debug, Clone)]
pub struct InsertedTrip {
    pub trip_id: i64,
    pub period: PgRange<DateTime<Utc>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub fiskeridir_vessel_id: i64,
}

impl TryFrom<Trip> for kyogre_core::Trip {
    type Error = Report<PostgresError>;

    fn try_from(value: Trip) -> Result<Self, Self::Error> {
        let period =
            DateRange::try_from(value.period).change_context(PostgresError::DataConversion)?;

        let landing_coverage = DateRange::try_from(value.landing_coverage)
            .change_context(PostgresError::DataConversion)?;

        let precision_period = value
            .period_precision
            .map(DateRange::try_from)
            .transpose()
            .change_context(PostgresError::DataConversion)?;

        let distance = value
            .distance
            .map(decimal_to_float)
            .transpose()
            .change_context(PostgresError::DataConversion)?;

        Ok(kyogre_core::Trip {
            trip_id: TripId(value.trip_id),
            period,
            landing_coverage,
            distance,
            assembler_id: value.trip_assembler_id,
            precision_period,
            start_port_code: value.start_port_id,
            end_port_code: value.end_port_id,
        })
    }
}

impl TryFrom<CurrentTrip> for kyogre_core::CurrentTrip {
    type Error = Report<PostgresError>;

    fn try_from(v: CurrentTrip) -> Result<Self, Self::Error> {
        Ok(Self {
            departure: v.departure_timestamp,
            target_species_fiskeridir_id: v.target_species_fiskeridir_id,
            hauls: serde_json::from_str::<Vec<TripHaul>>(&v.hauls)
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::TripHaul::try_from)
                .collect::<Result<_, _>>()?,
            fishing_facilities: serde_json::from_str::<Vec<FishingFacility>>(&v.fishing_facilities)
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::FishingFacility::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<TripCalculationTimer> for kyogre_core::TripCalculationTimer {
    fn from(v: TripCalculationTimer) -> Self {
        Self {
            fiskeridir_vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id),
            timestamp: v.timestamp,
            queued_reset: v.queued_reset,
            conflict: v.conflict,
        }
    }
}
impl TryFrom<TripDetailed> for kyogre_core::TripDetailed {
    type Error = Report<PostgresError>;

    fn try_from(value: TripDetailed) -> Result<Self, Self::Error> {
        let period =
            DateRange::try_from(value.period).change_context(PostgresError::DataConversion)?;
        let period_precision = value
            .period_precision
            .map(DateRange::try_from)
            .transpose()
            .change_context(PostgresError::DataConversion)?;

        let landing_coverage = DateRange::try_from(value.landing_coverage)
            .change_context(PostgresError::DataConversion)?;

        let mut vessel_events = serde_json::from_str::<Vec<VesselEvent>>(&value.vessel_events)
            .change_context(PostgresError::DataConversion)?
            .into_iter()
            .map(kyogre_core::VesselEvent::from)
            .collect::<Vec<kyogre_core::VesselEvent>>();

        let landing_ids = value
            .landing_ids
            .into_iter()
            .map(LandingId::try_from)
            .collect::<error_stack::Result<Vec<LandingId>, _>>()
            .change_context(PostgresError::DataConversion)?;

        vessel_events.sort_by_key(|v| v.report_timestamp);

        Ok(kyogre_core::TripDetailed {
            period_precision,
            fiskeridir_vessel_id: FiskeridirVesselId(value.fiskeridir_vessel_id),
            fiskeridir_length_group_id: value.fiskeridir_length_group_id,
            landing_coverage,
            trip_id: TripId(value.trip_id),
            period,
            num_deliveries: value.num_deliveries as u32,
            most_recent_delivery_date: value.latest_landing_timestamp,
            gear_ids: value.gear_ids,
            gear_group_ids: value.gear_group_ids,
            species_group_ids: value.species_group_ids,
            delivery_point_ids: value
                .delivery_points
                .into_iter()
                .map(|v| DeliveryPointId::try_from(v).change_context(PostgresError::DataConversion))
                .collect::<Result<_, _>>()?,
            hauls: serde_json::from_str::<Vec<TripHaul>>(&value.hauls)
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::TripHaul::try_from)
                .collect::<Result<_, _>>()?,
            fishing_facilities: serde_json::from_str::<Vec<FishingFacility>>(
                &value.fishing_facilities,
            )
            .change_context(PostgresError::DataConversion)?
            .into_iter()
            .map(kyogre_core::FishingFacility::try_from)
            .collect::<Result<_, _>>()?,
            delivery: kyogre_core::Delivery {
                delivered: serde_json::from_str::<Vec<Catch>>(&value.catches)
                    .change_context(PostgresError::DataConversion)?
                    .into_iter()
                    .map(kyogre_core::Catch::from)
                    .collect::<Vec<kyogre_core::Catch>>(),
                total_living_weight: decimal_to_float(value.total_living_weight)
                    .change_context(PostgresError::DataConversion)?,
                total_gross_weight: decimal_to_float(value.total_gross_weight)
                    .change_context(PostgresError::DataConversion)?,
                total_product_weight: decimal_to_float(value.total_product_weight)
                    .change_context(PostgresError::DataConversion)?,
            },
            start_port_id: value.start_port_id,
            end_port_id: value.end_port_id,
            assembler_id: value.trip_assembler_id,
            vessel_events,
            landing_ids,
            distance: value
                .distance
                .map(|v| decimal_to_float(v).change_context(PostgresError::DataConversion))
                .transpose()?,
            cache_version: value.cache_version,
        })
    }
}

impl From<VesselEvent> for kyogre_core::VesselEvent {
    fn from(v: VesselEvent) -> Self {
        kyogre_core::VesselEvent {
            event_id: v.vessel_event_id as u64,
            vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id),
            report_timestamp: v.report_timestamp,
            event_type: v.vessel_event_type_id,
            occurence_timestamp: v.occurence_timestamp,
        }
    }
}

impl From<Delivery> for kyogre_core::Delivery {
    fn from(d: Delivery) -> Self {
        kyogre_core::Delivery {
            delivered: d
                .catches
                .into_iter()
                .map(kyogre_core::Catch::from)
                .collect(),
            total_living_weight: d.total_living_weight,
            total_product_weight: d.total_product_weight,
            total_gross_weight: d.total_gross_weight,
        }
    }
}

impl From<TripAssemblerConflict> for kyogre_core::TripAssemblerConflict {
    fn from(v: TripAssemblerConflict) -> Self {
        Self {
            fiskeridir_vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id),
            timestamp: v.timestamp,
        }
    }
}
impl From<Catch> for kyogre_core::Catch {
    fn from(c: Catch) -> Self {
        kyogre_core::Catch {
            living_weight: c.living_weight,
            gross_weight: c.gross_weight,
            product_weight: c.product_weight,
            species_fiskeridir_id: c.species_fiskeridir_id,
            product_quality_id: c.product_quality_id,
            product_quality_name: c.product_quality_id.name().to_owned(),
        }
    }
}

impl TryFrom<TripHaul> for kyogre_core::TripHaul {
    type Error = Report<PostgresError>;

    fn try_from(v: TripHaul) -> Result<Self, Self::Error> {
        Ok(Self {
            haul_id: HaulId(v.haul_id),
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            start_latitude: decimal_to_float(v.start_latitude)
                .change_context(PostgresError::DataConversion)?,
            start_longitude: decimal_to_float(v.start_longitude)
                .change_context(PostgresError::DataConversion)?,
            start_timestamp: v.start_timestamp,
            stop_latitude: decimal_to_float(v.stop_latitude)
                .change_context(PostgresError::DataConversion)?,
            stop_longitude: decimal_to_float(v.stop_longitude)
                .change_context(PostgresError::DataConversion)?,
            stop_timestamp: v.stop_timestamp,
            total_living_weight: v.total_living_weight,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            catches: v
                .catches
                .into_iter()
                .map(kyogre_core::HaulCatch::try_from)
                .collect::<Result<_, _>>()?,
            whale_catches: v
                .whale_catches
                .into_iter()
                .map(kyogre_core::WhaleCatch::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}
