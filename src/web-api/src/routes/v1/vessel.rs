use crate::error::error::UpdateVesselNotFoundSnafu;
use crate::{
    Database,
    error::Result,
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response,
};
use actix_web::web::{self};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, GearGroup, RegisterVesselOwner, SpeciesGroup, VesselLengthGroup};
use futures::TryStreamExt;
use kyogre_core::{
    DEFAULT_LIVE_FUEL_THRESHOLD, EngineType, FiskeridirVesselId, FuelQuery, LiveFuelQuery, Mmsi,
    NaiveDateRange, VesselCurrentTrip,
};
use kyogre_core::{LiveFuel, UpdateVessel};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{DisplayFromStr, serde_as};

pub mod benchmarks;

#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelParams {
    #[serde(flatten)]
    pub range: NaiveDateRange<30>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveFuelParams {
    pub threshold: Option<DateTime<Utc>>,
}

/// Updates the vessel with the provided information.
/// Note that all trip benchmarks that rely on some of the provided information will not be
/// updated immediatley upon updating a vessel, trip benchmark updates can be expected within 24 hours.
#[oasgen(skip(db), tags("Vessel"))]
pub async fn update_vessel<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    bw_profile: BwProfile,
    update: web::Json<UpdateVessel>,
) -> Result<Response<Vessel>> {
    let cs = bw_profile.call_sign()?;

    Ok(Response::new(
        db.update_vessel(cs, &update)
            .await?
            .ok_or_else(|| {
                UpdateVesselNotFoundSnafu {
                    call_sign: cs.clone(),
                }
                .build()
            })?
            .into(),
    ))
}

/// Returns all known vessels.
#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db))]
pub async fn vessels<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<Vessel> {
    stream_response! {
        db.vessels().map_ok(Vessel::from)
    }
}

/// Returns a fuel consumption estimate for the given date range for the vessel associated with the
/// authenticated user, if no date range is given the last 30 days
/// are returned.
/// This is not based on trips and is the full fuel consumption estimate for the given date range
#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db))]
pub async fn fuel<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FuelParams>,
) -> Result<Response<f64>> {
    let call_sign = profile.into_call_sign()?;
    let query = params.into_inner().to_query(call_sign);

    Ok(Response::new(db.fuel_estimation(&query).await?))
}

#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db))]
pub async fn live_fuel<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<LiveFuelParams>,
) -> Result<Response<LiveFuel>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().to_query(call_sign.clone());

    Ok(Response::new(db.live_fuel(&query).await?))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vessel {
    pub fiskeridir: FiskeridirVessel,
    pub ais: Option<AisVessel>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub gear_groups: Vec<GearGroup>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub species_groups: Vec<SpeciesGroup>,
    pub current_trip: Option<VesselCurrentTrip>,
    pub is_active: bool,
}

impl Vessel {
    pub fn mmsi(&self) -> Option<Mmsi> {
        self.ais.as_ref().map(|v| v.mmsi)
    }
    pub fn ais_call_sign(&self) -> Option<&str> {
        self.ais.as_ref().and_then(|v| v.call_sign.as_deref())
    }
    pub fn fiskeridir_call_sign(&self) -> Option<&str> {
        self.fiskeridir.call_sign.as_ref().map(|v| v.as_ref())
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiskeridirVessel {
    pub id: FiskeridirVesselId,
    #[serde_as(as = "DisplayFromStr")]
    pub length_group_id: VesselLengthGroup,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owners: Vec<RegisterVesselOwner>,
    pub engine_building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub building_year: Option<u32>,
    pub auxiliary_engine_power: Option<u32>,
    pub auxiliary_engine_building_year: Option<u32>,
    pub boiler_engine_power: Option<u32>,
    pub boiler_engine_building_year: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub engine_type: Option<EngineType>,
    pub engine_rpm: Option<u32>,
    pub degree_of_electrification: Option<f64>,
    pub service_speed: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AisVessel {
    pub mmsi: Mmsi,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
}
impl LiveFuelParams {
    pub fn to_query(self, call_sign: CallSign) -> LiveFuelQuery {
        let Self { threshold } = self;

        LiveFuelQuery {
            call_sign,
            threshold: threshold.unwrap_or_else(|| Utc::now() - DEFAULT_LIVE_FUEL_THRESHOLD),
        }
    }
}

impl FuelParams {
    pub fn to_query(self, call_sign: CallSign) -> FuelQuery {
        let Self { range } = self;
        FuelQuery {
            call_sign,
            start_date: range.start(),
            end_date: range.end(),
        }
    }
}

impl From<kyogre_core::Vessel> for Vessel {
    fn from(value: kyogre_core::Vessel) -> Self {
        let kyogre_core::Vessel {
            fiskeridir,
            ais,
            preferred_trip_assembler: _,
            gear_groups,
            species_groups,
            current_trip,
            is_active,
        } = value;

        Vessel {
            fiskeridir: FiskeridirVessel::from(fiskeridir),
            ais: ais.map(AisVessel::from),
            gear_groups,
            species_groups,
            current_trip,
            is_active,
        }
    }
}

impl From<kyogre_core::AisVessel> for AisVessel {
    fn from(value: kyogre_core::AisVessel) -> Self {
        let kyogre_core::AisVessel {
            mmsi,
            call_sign,
            name,
            length: _,
            breadth: _,
            current_draught: _,
        } = value;

        AisVessel {
            mmsi,
            call_sign,
            name,
        }
    }
}

impl From<kyogre_core::FiskeridirVessel> for FiskeridirVessel {
    fn from(value: kyogre_core::FiskeridirVessel) -> Self {
        let kyogre_core::FiskeridirVessel {
            id,
            length_group_id,
            call_sign,
            name,
            registration_id,
            length,
            width,
            owners,
            engine_building_year,
            engine_power,
            building_year,
            auxiliary_engine_power,
            boiler_engine_power,
            auxiliary_engine_building_year,
            boiler_engine_building_year,
            engine_type,
            engine_rpm,
            engine_version: _,
            degree_of_electrification,
            service_speed,
        } = value;

        FiskeridirVessel {
            id,
            length_group_id,
            call_sign,
            name,
            registration_id,
            length,
            width,
            owners,
            engine_building_year,
            engine_power,
            building_year,
            auxiliary_engine_power,
            auxiliary_engine_building_year,
            boiler_engine_power,
            boiler_engine_building_year,
            engine_type,
            engine_rpm,
            degree_of_electrification,
            service_speed,
        }
    }
}

impl PartialEq<fiskeridir_rs::Vessel> for FiskeridirVessel {
    fn eq(&self, other: &fiskeridir_rs::Vessel) -> bool {
        let Self {
            id,
            length_group_id,
            call_sign,
            name,
            registration_id,
            length,
            width: _,
            owners: _,
            engine_building_year,
            engine_power,
            building_year,
            auxiliary_engine_power: _,
            boiler_engine_power: _,
            auxiliary_engine_building_year: _,
            boiler_engine_building_year: _,
            engine_type: _,
            engine_rpm: _,
            degree_of_electrification: _,
            service_speed: _,
        } = self;

        Some(*id) == other.id
            && Some(*length_group_id) == other.length_group_code
            && call_sign.as_ref().map(|v| v.as_ref())
                == other.call_sign.as_ref().map(|v| v.as_ref())
            && name.as_deref() == other.name.as_deref()
            && registration_id.as_deref() == other.registration_id.as_deref()
            && length.map(|v| v as u32) == other.length.map(|v| v as u32)
            && *engine_building_year == other.engine_building_year
            && *engine_power == other.engine_power
            && *building_year == other.building_year
    }
}

impl PartialEq<kyogre_core::AisVessel> for AisVessel {
    fn eq(&self, other: &kyogre_core::AisVessel) -> bool {
        let Self {
            mmsi,
            call_sign,
            name,
        } = self;

        *mmsi == other.mmsi && *call_sign == other.call_sign && *name == other.name
    }
}

impl PartialEq<kyogre_core::FiskeridirVessel> for FiskeridirVessel {
    fn eq(&self, other: &kyogre_core::FiskeridirVessel) -> bool {
        let Self {
            id,
            length_group_id,
            call_sign,
            name,
            registration_id,
            length,
            width: _,
            owners: _,
            engine_building_year,
            engine_power,
            building_year,
            auxiliary_engine_power,
            boiler_engine_power,
            auxiliary_engine_building_year,
            boiler_engine_building_year,
            engine_type,
            engine_rpm,
            degree_of_electrification,
            service_speed,
        } = self;

        *id == other.id
            && *length_group_id == other.length_group_id
            && *call_sign == other.call_sign
            && *name == other.name
            && *registration_id == other.registration_id
            && *length == other.length
            && *engine_building_year == other.engine_building_year
            && *engine_power == other.engine_power
            && *building_year == other.building_year
            && *auxiliary_engine_power == other.auxiliary_engine_power
            && *auxiliary_engine_building_year == other.auxiliary_engine_building_year
            && *boiler_engine_power == other.boiler_engine_power
            && *boiler_engine_building_year == other.boiler_engine_building_year
            && *engine_type == other.engine_type
            && *engine_rpm == other.engine_rpm
            && *degree_of_electrification == other.degree_of_electrification
            && *service_speed == other.service_speed
    }
}

impl PartialEq<FiskeridirVessel> for kyogre_core::FiskeridirVessel {
    fn eq(&self, other: &FiskeridirVessel) -> bool {
        other.eq(self)
    }
}

impl PartialEq<AisVessel> for kyogre_core::AisVessel {
    fn eq(&self, other: &AisVessel) -> bool {
        other.eq(self)
    }
}

impl PartialEq<FiskeridirVessel> for fiskeridir_rs::Vessel {
    fn eq(&self, other: &FiskeridirVessel) -> bool {
        other.eq(self)
    }
}

impl PartialEq<UpdateVessel> for Vessel {
    fn eq(&self, other: &UpdateVessel) -> bool {
        let UpdateVessel {
            engine_power,
            engine_building_year,
            auxiliary_engine_power,
            boiler_engine_power,
            auxiliary_engine_building_year,
            boiler_engine_building_year,
            engine_type,
            engine_rpm,
            degree_of_electrification,
            service_speed,
        } = other;

        self.fiskeridir.engine_power == *engine_power
            && self.fiskeridir.engine_building_year == *engine_building_year
            && self.fiskeridir.auxiliary_engine_power == *auxiliary_engine_power
            && self.fiskeridir.auxiliary_engine_building_year == *auxiliary_engine_building_year
            && self.fiskeridir.boiler_engine_power == *boiler_engine_power
            && self.fiskeridir.boiler_engine_building_year == *boiler_engine_building_year
            && self.fiskeridir.engine_type == *engine_type
            && self.fiskeridir.engine_rpm == *engine_rpm
            && self.fiskeridir.degree_of_electrification == *degree_of_electrification
            && self.fiskeridir.service_speed == *service_speed
    }
}

impl PartialEq<Vessel> for UpdateVessel {
    fn eq(&self, other: &Vessel) -> bool {
        other.eq(self)
    }
}
