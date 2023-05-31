use super::{HaulCatch, WhaleCatch};
use crate::{error::PostgresError, queries::decimal_to_float};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, Quality, VesselLengthGroup};
use kyogre_core::{
    CatchLocationId, DateRange, FiskeridirVesselId, HaulId, TripAssemblerId, TripId,
    VesselEventType,
};
use num_traits::FromPrimitive;
use serde::Deserialize;
use sqlx::postgres::types::PgRange;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: i64,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub trip_assembler_id: TripAssemblerId,
}

#[derive(Debug, Clone)]
pub struct TripCalculationTimer {
    pub timestamp: DateTime<Utc>,
    pub fiskeridir_vessel_id: i64,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerConflict {
    pub fiskeridir_vessel_id: i64,
    pub timestamp: DateTime<Utc>,
}

pub struct TripDetailed {
    pub trip_id: i64,
    pub fiskeridir_vessel_id: i64,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub num_deliveries: i64,
    pub total_living_weight: BigDecimal,
    pub total_gross_weight: BigDecimal,
    pub total_product_weight: BigDecimal,
    pub delivery_points: Vec<String>,
    pub gear_ids: Vec<Gear>,
    pub latest_landing_timestamp: Option<DateTime<Utc>>,
    pub catches: String,
    pub hauls: String,
    pub vessel_events: String,
    pub delivery_point_catches: String,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub trip_assembler_id: TripAssemblerId,
}

#[derive(Deserialize)]
struct TripHaul {
    haul_id: i64,
    ers_activity_id: String,
    duration: i32,
    haul_distance: Option<i32>,
    catch_location_start: Option<String>,
    ocean_depth_end: i32,
    ocean_depth_start: i32,
    quota_type_id: i32,
    start_timestamp: DateTime<Utc>,
    stop_timestamp: DateTime<Utc>,
    start_latitude: BigDecimal,
    start_longitude: BigDecimal,
    stop_latitude: BigDecimal,
    stop_longitude: BigDecimal,
    gear_id: Gear,
    gear_group_id: GearGroup,
    fiskeridir_vessel_id: Option<i64>,
    vessel_call_sign: Option<String>,
    vessel_call_sign_ers: String,
    vessel_length: BigDecimal,
    vessel_length_group: VesselLengthGroup,
    vessel_name: Option<String>,
    vessel_name_ers: Option<String>,
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
    product_quality_id: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct VesselEvent {
    vessel_event_id: i64,
    fiskeridir_vessel_id: i64,
    timestamp: DateTime<Utc>,
    vessel_event_type_id: VesselEventType,
}

#[derive(Debug, Clone, Deserialize)]
struct DeliveryPointCatch {
    delivery_point_id: String,
    #[serde(flatten)]
    delivery: Delivery,
}

impl TryFrom<Trip> for kyogre_core::Trip {
    type Error = Report<PostgresError>;

    fn try_from(value: Trip) -> Result<Self, Self::Error> {
        let period = DateRange::try_from(value.period)
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        let landing_coverage = DateRange::try_from(value.landing_coverage)
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        let precision_period = value
            .period_precision
            .map(DateRange::try_from)
            .transpose()
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        Ok(kyogre_core::Trip {
            trip_id: TripId(value.trip_id),
            period,
            landing_coverage,
            assembler_id: value.trip_assembler_id,
            precision_period,
        })
    }
}

impl From<TripCalculationTimer> for kyogre_core::TripCalculationTimer {
    fn from(v: TripCalculationTimer) -> Self {
        Self {
            fiskeridir_vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id),
            timestamp: v.timestamp,
        }
    }
}
impl TryFrom<TripDetailed> for kyogre_core::TripDetailed {
    type Error = Report<PostgresError>;

    fn try_from(value: TripDetailed) -> Result<Self, Self::Error> {
        let period = DateRange::try_from(value.period)
            .into_report()
            .change_context(PostgresError::DataConversion)?;
        let period_precision = value
            .period_precision
            .map(DateRange::try_from)
            .transpose()
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        let landing_coverage = DateRange::try_from(value.landing_coverage)
            .into_report()
            .change_context(PostgresError::DataConversion)?;

        let db_delivery_point_catches: Vec<DeliveryPointCatch> =
            serde_json::from_str(&value.delivery_point_catches)
                .into_report()
                .change_context(PostgresError::DataConversion)?;

        let mut delivery_point_catches = HashMap::with_capacity(db_delivery_point_catches.len());
        for d in db_delivery_point_catches {
            let delivery_point_id = DeliveryPointId::try_from(d.delivery_point_id)
                .change_context(PostgresError::DataConversion)?;
            let delivery = kyogre_core::Delivery::from(d.delivery);

            delivery_point_catches.insert(delivery_point_id, delivery);
        }

        Ok(kyogre_core::TripDetailed {
            period_precision,
            fiskeridir_vessel_id: FiskeridirVesselId(value.fiskeridir_vessel_id),
            landing_coverage,
            trip_id: TripId(value.trip_id),
            period,
            num_deliveries: value.num_deliveries as u32,
            most_recent_delivery_date: value.latest_landing_timestamp,
            gear_ids: value.gear_ids,
            delivery_point_ids: value
                .delivery_points
                .into_iter()
                .map(|v| DeliveryPointId::try_from(v).change_context(PostgresError::DataConversion))
                .collect::<Result<_, _>>()?,
            hauls: serde_json::from_str::<Vec<TripHaul>>(&value.hauls)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::Haul::try_from)
                .collect::<Result<_, _>>()?,
            delivery: kyogre_core::Delivery {
                delivered: serde_json::from_str::<Vec<Catch>>(&value.catches)
                    .into_report()
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
            delivered_per_delivery_point: delivery_point_catches,
            assembler_id: value.trip_assembler_id,
            vessel_events: serde_json::from_str::<Vec<VesselEvent>>(&value.vessel_events)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::VesselEvent::from)
                .collect::<Vec<kyogre_core::VesselEvent>>(),
        })
    }
}

impl From<VesselEvent> for kyogre_core::VesselEvent {
    fn from(v: VesselEvent) -> Self {
        kyogre_core::VesselEvent {
            event_id: v.vessel_event_id as u64,
            vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id),
            timestamp: v.timestamp,
            event_type: v.vessel_event_type_id,
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
        // TODO: remove when fiskeridir_rs has enum sqlx support
        let product_quality = Quality::from_i32(c.product_quality_id).unwrap();
        kyogre_core::Catch {
            living_weight: c.living_weight,
            gross_weight: c.gross_weight,
            product_weight: c.product_weight,
            species_fiskeridir_id: c.species_fiskeridir_id,
            product_quality_id: c.product_quality_id,
            product_quality_name: product_quality.name().to_owned(),
        }
    }
}

impl TryFrom<TripHaul> for kyogre_core::Haul {
    type Error = Report<PostgresError>;

    fn try_from(v: TripHaul) -> Result<Self, Self::Error> {
        Ok(Self {
            haul_id: HaulId(v.haul_id),
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_location_start: v
                .catch_location_start
                .map(CatchLocationId::try_from)
                .transpose()
                .change_context(PostgresError::DataConversion)?,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
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
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: decimal_to_float(v.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_length_group: v.vessel_length_group,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
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
