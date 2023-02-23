use crate::AisVessel;

#[derive(Debug, Clone)]
pub struct Vessel {
    pub fiskeridir: FiskeridirVessel,
    pub ais: Option<AisVessel>,
}

#[derive(Debug, Clone)]
pub struct FiskeridirVessel {
    pub id: i64,
    pub vessel_type_id: Option<u32>,
    pub length_group_id: Option<u32>,
    pub nation_group_id: String,
    pub nation_id: String,
    pub norwegian_municipality_id: Option<u32>,
    pub norwegian_county_id: Option<u32>,
    pub gross_tonnage_1969: Option<u32>,
    pub gross_tonnage_other: Option<u32>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owner: Option<String>,
    pub engine_building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub building_year: Option<u32>,
    pub rebuilding_year: Option<u32>,
}

impl Vessel {
    pub fn mmsi(&self) -> Option<i32> {
        self.ais.as_ref().map(|v| v.mmsi)
    }
}
