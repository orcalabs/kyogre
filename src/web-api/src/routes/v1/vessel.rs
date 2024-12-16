use crate::error::error::{MissingDateRangeSnafu, OrgNotFoundSnafu, UpdateVesselNotFoundSnafu};
use crate::{
    error::Result,
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response, Database,
};
use actix_web::web::{self, Path};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::{
    CallSign, GearGroup, OrgId, RegisterVesselOwner, SpeciesGroup, VesselLengthGroup,
};
use futures::TryStreamExt;
use kyogre_core::{FiskeridirVesselId, Mmsi, OrgBenchmarkQuery, OrgBenchmarks};
use kyogre_core::{UpdateVessel, VesselBenchmarks};
use oasgen::{oasgen, OaSchema};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrgBenchmarkParameters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, OaSchema, Deserialize)]
pub struct OrgBenchmarkPath {
    pub org_id: OrgId,
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

/// Returns organization benchmarks for the given organization id (Breg org id).
/// This will include benchmarks for all vessels associated with the organization.
#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db))]
pub async fn org_benchmarks<T: Database + 'static>(
    db: web::Data<T>,
    bw_profile: BwProfile,
    params: Query<OrgBenchmarkParameters>,
    path: Path<OrgBenchmarkPath>,
) -> Result<Response<OrgBenchmarks>> {
    let call_sign = bw_profile.call_sign()?;
    let query = params
        .into_inner()
        .into_query(call_sign.clone(), path.org_id)?;

    match db.org_benchmarks(&query).await? {
        Some(b) => Ok(Response::new(b)),
        None => OrgNotFoundSnafu {
            org_id: path.org_id,
        }
        .fail(),
    }
}

/// Returns benchmark data for the vessel associated with the authenticated user.
#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db))]
pub async fn vessel_benchmarks<T: Database + 'static>(
    db: web::Data<T>,
    bw_profile: BwProfile,
) -> Result<Response<VesselBenchmarks>> {
    let call_sign = bw_profile.call_sign()?;
    Ok(Response::new(
        db.vessel_benchmarks(&bw_profile.user.id, call_sign).await?,
    ))
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
    pub vessel_type_id: Option<u32>,
    #[serde_as(as = "DisplayFromStr")]
    pub length_group_id: VesselLengthGroup,
    pub nation_group_id: Option<String>,
    pub nation_id: Option<String>,
    pub norwegian_municipality_id: Option<u32>,
    pub norwegian_county_id: Option<u32>,
    pub gross_tonnage_1969: Option<u32>,
    pub gross_tonnage_other: Option<u32>,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owner: Option<String>,
    pub owners: Vec<RegisterVesselOwner>,
    pub engine_building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub building_year: Option<u32>,
    pub rebuilding_year: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AisVessel {
    pub mmsi: Mmsi,
    pub imo_number: Option<i32>,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub destination: Option<String>,
}

impl OrgBenchmarkParameters {
    pub fn into_query(self, call_sign: CallSign, org_id: OrgId) -> Result<OrgBenchmarkQuery> {
        let (start, end) = match (self.start, self.end) {
            (None, None) => {
                let now = Utc::now();
                Ok((now - Duration::days(30), now))
            }
            (Some(s), Some(e)) => Ok((s, e)),
            _ => MissingDateRangeSnafu {
                start: self.start.is_some(),
                end: self.end.is_some(),
            }
            .fail(),
        }?;
        Ok(OrgBenchmarkQuery {
            start,
            end,
            call_sign,
            org_id,
        })
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
        } = value;

        Vessel {
            fiskeridir: FiskeridirVessel::from(fiskeridir),
            ais: ais.map(AisVessel::from),
            gear_groups,
            species_groups,
        }
    }
}

impl From<kyogre_core::AisVessel> for AisVessel {
    fn from(value: kyogre_core::AisVessel) -> Self {
        let kyogre_core::AisVessel {
            mmsi,
            imo_number,
            call_sign,
            name,
            ship_length,
            ship_width,
            eta,
            destination,
        } = value;

        AisVessel {
            mmsi,
            imo_number,
            call_sign,
            name,
            ship_length,
            ship_width,
            eta,
            destination,
        }
    }
}

impl From<kyogre_core::FiskeridirVessel> for FiskeridirVessel {
    fn from(value: kyogre_core::FiskeridirVessel) -> Self {
        let kyogre_core::FiskeridirVessel {
            id,
            vessel_type_id,
            length_group_id,
            nation_group_id,
            nation_id,
            norwegian_municipality_id,
            norwegian_county_id,
            gross_tonnage_1969,
            gross_tonnage_other,
            call_sign,
            name,
            registration_id,
            length,
            width,
            owner,
            owners,
            engine_building_year,
            engine_power,
            building_year,
            rebuilding_year,
        } = value;

        FiskeridirVessel {
            id,
            vessel_type_id,
            length_group_id,
            nation_group_id,
            nation_id,
            norwegian_municipality_id,
            norwegian_county_id,
            gross_tonnage_1969,
            gross_tonnage_other,
            call_sign,
            name,
            registration_id,
            length,
            width,
            owner,
            owners,
            engine_building_year,
            engine_power,
            building_year,
            rebuilding_year,
        }
    }
}

impl PartialEq<fiskeridir_rs::Vessel> for FiskeridirVessel {
    fn eq(&self, other: &fiskeridir_rs::Vessel) -> bool {
        let Self {
            id,
            vessel_type_id,
            length_group_id,
            nation_group_id,
            nation_id,
            norwegian_municipality_id,
            norwegian_county_id,
            gross_tonnage_1969,
            gross_tonnage_other,
            call_sign,
            name,
            registration_id,
            length,
            width: _,
            owner: _,
            owners: _,
            engine_building_year,
            engine_power,
            building_year,
            rebuilding_year,
        } = self;

        Some(*id) == other.id
            && *vessel_type_id == other.type_code.map(|v| v as u32)
            && Some(*length_group_id) == other.length_group_code
            && nation_group_id.as_deref() == other.nationality_group.as_deref()
            && *norwegian_municipality_id == other.municipality_code
            && *norwegian_county_id == other.county_code
            && *gross_tonnage_1969 == other.gross_tonnage_1969
            && *gross_tonnage_other == other.gross_tonnage_other
            && call_sign.as_ref().map(|v| v.as_ref())
                == other.call_sign.as_ref().map(|v| v.as_ref())
            && name.as_deref() == other.name.as_deref()
            && registration_id.as_deref() == other.registration_id.as_deref()
            && length.map(|v| v as u32) == other.length.map(|v| v as u32)
            && *engine_building_year == other.engine_building_year
            && *engine_power == other.engine_power
            && *building_year == other.building_year
            && *rebuilding_year == other.rebuilding_year
            && *nation_id == Some(other.nationality_code.alpha3().to_string())
    }
}

impl PartialEq<kyogre_core::AisVessel> for AisVessel {
    fn eq(&self, other: &kyogre_core::AisVessel) -> bool {
        let Self {
            mmsi,
            imo_number,
            call_sign,
            name,
            ship_length,
            ship_width,
            eta,
            destination,
        } = self;

        *mmsi == other.mmsi
            && *call_sign == other.call_sign
            && *imo_number == other.imo_number
            && *name == other.name
            && *ship_length == other.ship_length
            && *ship_width == other.ship_width
            && *eta == other.eta
            && *destination == other.destination
    }
}

impl PartialEq<kyogre_core::FiskeridirVessel> for FiskeridirVessel {
    fn eq(&self, other: &kyogre_core::FiskeridirVessel) -> bool {
        let Self {
            id,
            vessel_type_id,
            length_group_id,
            nation_group_id,
            nation_id,
            norwegian_municipality_id,
            norwegian_county_id,
            gross_tonnage_1969,
            gross_tonnage_other,
            call_sign,
            name,
            registration_id,
            length,
            width: _,
            owner: _,
            owners: _,
            engine_building_year,
            engine_power,
            building_year,
            rebuilding_year,
        } = self;

        *id == other.id
            && *vessel_type_id == other.vessel_type_id
            && *length_group_id == other.length_group_id
            && *nation_group_id == other.nation_group_id
            && *nation_id == other.nation_id
            && *norwegian_municipality_id == other.norwegian_municipality_id
            && *norwegian_county_id == other.norwegian_county_id
            && *gross_tonnage_1969 == other.gross_tonnage_1969
            && *gross_tonnage_other == other.gross_tonnage_other
            && *call_sign == other.call_sign
            && *name == other.name
            && *registration_id == other.registration_id
            && *length == other.length
            && *engine_building_year == other.engine_building_year
            && *engine_power == other.engine_power
            && *building_year == other.building_year
            && *rebuilding_year == other.rebuilding_year
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
        } = other;

        self.fiskeridir.engine_power == *engine_power
            && self.fiskeridir.engine_building_year == *engine_building_year
    }
}

impl PartialEq<Vessel> for UpdateVessel {
    fn eq(&self, other: &Vessel) -> bool {
        other.eq(self)
    }
}
