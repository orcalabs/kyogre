use super::{FishingFacility, HaulCatch, HaulOceanClimate, HaulWeather, WhaleCatch};
use crate::{error::PostgresError, queries::decimal_to_float};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, Quality, VesselLengthGroup};
use kyogre_core::{
    CatchLocationId, DateRange, FiskeridirVesselId, HaulId, TripAssemblerId, TripId,
    VesselEventType,
};
use serde::Deserialize;
use sqlx::postgres::types::PgRange;

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: i64,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub landing_coverage: PgRange<DateTime<Utc>>,
    pub distance: Option<BigDecimal>,
    pub trip_assembler_id: TripAssemblerId,
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
    pub landing_ids: Vec<String>,
    pub latest_landing_timestamp: Option<DateTime<Utc>>,
    pub catches: String,
    pub hauls: String,
    pub fishing_facilities: String,
    pub vessel_events: String,
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
    catch_locations: Option<Vec<String>>,
    ocean_depth_end: i32,
    ocean_depth_start: i32,
    quota_type_id: i32,
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
    vessel_call_sign: Option<String>,
    vessel_call_sign_ers: String,
    vessel_length: BigDecimal,
    vessel_length_group: VesselLengthGroup,
    vessel_name: Option<String>,
    vessel_name_ers: Option<String>,
    wind_speed_10m: Option<BigDecimal>,
    wind_direction_10m: Option<BigDecimal>,
    air_temperature_2m: Option<BigDecimal>,
    relative_humidity_2m: Option<BigDecimal>,
    air_pressure_at_sea_level: Option<BigDecimal>,
    precipitation_amount: Option<BigDecimal>,
    cloud_area_fraction: Option<BigDecimal>,
    water_speed: Option<BigDecimal>,
    water_direction: Option<BigDecimal>,
    salinity: Option<BigDecimal>,
    water_temperature: Option<BigDecimal>,
    ocean_climate_depth: Option<BigDecimal>,
    sea_floor_depth: Option<BigDecimal>,
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
                .map(kyogre_core::Haul::try_from)
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
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::Haul::try_from)
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
            catch_locations: v
                .catch_locations
                .map(|c| c.into_iter().map(CatchLocationId::try_from).collect())
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
            total_living_weight: v.total_living_weight,
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
            weather: HaulWeather {
                wind_speed_10m: v.wind_speed_10m,
                wind_direction_10m: v.wind_direction_10m,
                air_temperature_2m: v.air_temperature_2m,
                relative_humidity_2m: v.relative_humidity_2m,
                air_pressure_at_sea_level: v.air_pressure_at_sea_level,
                precipitation_amount: v.precipitation_amount,
                cloud_area_fraction: v.cloud_area_fraction,
            }
            .try_into()?,
            ocean_climate: HaulOceanClimate {
                water_speed: v.water_speed,
                water_direction: v.water_direction,
                salinity: v.salinity,
                water_temperature: v.water_temperature,
                ocean_climate_depth: v.ocean_climate_depth,
                sea_floor_depth: v.sea_floor_depth,
            }
            .try_into()?,
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
