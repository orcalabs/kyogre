use crate::{error::ApiError, to_streaming_response, Database};
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, GearGroup, RegisterVesselOwner, SpeciesGroup, VesselLengthGroup};
use futures::TryStreamExt;
use kyogre_core::{FiskeridirVesselId, Mmsi};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tracing::{event, Level};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/vessels",
    responses(
        (status = 200, description = "all vessels", body = [Vessel]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn vessels<T: Database + 'static>(db: web::Data<T>) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db.vessels().map_ok(Vessel::from).map_err(|e| {
            event!(Level::ERROR, "failed to retrieve vessels: {:?}", e);
            ApiError::InternalServerError
        })
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vessel {
    pub fiskeridir: FiskeridirVessel,
    pub ais: Option<AisVessel>,
    pub fish_caught_per_hour: Option<f64>,
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
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FiskeridirVessel {
    #[schema(value_type = i64)]
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
    #[schema(value_type = String)]
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owner: Option<String>,
    pub owners: Option<Vec<RegisterVesselOwner>>,
    pub engine_building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub building_year: Option<u32>,
    pub rebuilding_year: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AisVessel {
    #[schema(value_type = i32)]
    pub mmsi: Mmsi,
    pub imo_number: Option<i32>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
    #[schema(value_type = Option<String>, example = "2023-02-24T11:08:20.409416682Z")]
    pub eta: Option<DateTime<Utc>>,
    pub destination: Option<String>,
}

impl From<kyogre_core::Vessel> for Vessel {
    fn from(value: kyogre_core::Vessel) -> Self {
        Vessel {
            fiskeridir: FiskeridirVessel::from(value.fiskeridir),
            ais: value.ais.map(AisVessel::from),
            fish_caught_per_hour: value.fish_caught_per_hour,
            gear_groups: value.gear_groups,
            species_groups: value.species_groups,
        }
    }
}

impl From<kyogre_core::AisVessel> for AisVessel {
    fn from(value: kyogre_core::AisVessel) -> Self {
        AisVessel {
            mmsi: value.mmsi,
            imo_number: value.imo_number,
            call_sign: value.call_sign.map(|v| v.into_inner()),
            name: value.name,
            ship_length: value.ship_length,
            ship_width: value.ship_width,
            eta: value.eta,
            destination: value.destination,
        }
    }
}

impl From<kyogre_core::FiskeridirVessel> for FiskeridirVessel {
    fn from(value: kyogre_core::FiskeridirVessel) -> Self {
        FiskeridirVessel {
            id: value.id,
            vessel_type_id: value.vessel_type_id,
            length_group_id: value.length_group_id,
            nation_group_id: value.nation_group_id,
            nation_id: value.nation_id,
            norwegian_municipality_id: value.norwegian_municipality_id,
            norwegian_county_id: value.norwegian_county_id,
            gross_tonnage_1969: value.gross_tonnage_1969,
            gross_tonnage_other: value.gross_tonnage_other,
            call_sign: value.call_sign,
            name: value.name,
            registration_id: value.registration_id,
            length: value.length,
            width: value.width,
            owner: value.owner,
            owners: value.owners,
            engine_building_year: value.engine_building_year,
            engine_power: value.engine_power,
            building_year: value.building_year,
            rebuilding_year: value.rebuilding_year,
        }
    }
}

impl PartialEq<fiskeridir_rs::Vessel> for FiskeridirVessel {
    fn eq(&self, other: &fiskeridir_rs::Vessel) -> bool {
        self.id.0 == other.id.unwrap()
            && self.vessel_type_id == other.type_code.map(|v| v as u32)
            && self.length_group_id == other.length_group_code
            && self.nation_group_id == other.nation_group.clone()
            && self.norwegian_municipality_id == other.municipality_code
            && self.norwegian_county_id == other.county_code
            && self.gross_tonnage_1969 == other.gross_tonnage_1969
            && self.gross_tonnage_other == other.gross_tonnage_other
            && self.call_sign.as_ref().map(|v| v.as_ref())
                == other.call_sign.as_ref().map(|v| v.as_ref())
            && self.name == other.name
            && self.registration_id == other.registration_id
            && self.length.map(|v| v as u32) == other.length.map(|v| v as u32)
            && self.engine_building_year == other.engine_building_year
            && self.engine_power == other.engine_power
            && self.building_year == other.building_year
            && self.rebuilding_year == other.rebuilding_year
            && self.nation_id == Some(other.nationality_code.alpha3().to_string())
    }
}

impl PartialEq<kyogre_core::AisVessel> for AisVessel {
    fn eq(&self, other: &kyogre_core::AisVessel) -> bool {
        self.mmsi == other.mmsi
            && self.call_sign.as_ref().map(|v| v.as_ref())
                == other.call_sign.as_ref().map(|v| v.as_ref())
            && self.imo_number == other.imo_number
            && self.name == other.name
            && self.ship_length == other.ship_length
            && self.ship_width == other.ship_width
            && self.eta == other.eta
            && self.destination == other.destination
    }
}

impl PartialEq<kyogre_core::FiskeridirVessel> for FiskeridirVessel {
    fn eq(&self, other: &kyogre_core::FiskeridirVessel) -> bool {
        self.id == other.id
            && self.vessel_type_id == other.vessel_type_id
            && self.length_group_id == other.length_group_id
            && self.nation_group_id == other.nation_group_id
            && self.norwegian_municipality_id == other.norwegian_municipality_id
            && self.norwegian_county_id == other.norwegian_county_id
            && self.gross_tonnage_1969 == other.gross_tonnage_1969
            && self.gross_tonnage_other == other.gross_tonnage_other
            && self.call_sign == other.call_sign
            && self.name == other.name
            && self.registration_id == other.registration_id
            && self.length == other.length
            && self.engine_building_year == other.engine_building_year
            && self.engine_power == other.engine_power
            && self.building_year == other.building_year
            && self.rebuilding_year == other.rebuilding_year
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
