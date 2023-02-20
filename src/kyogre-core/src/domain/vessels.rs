use fiskeridir_rs::CallSign;

#[derive(Debug, Clone)]
pub struct Vessel {
    pub id: i64,
    pub mmsi: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewFiskeridirVessel {
    pub id: i64,
    pub call_sign: Option<CallSign>,
    pub registration_id: String,
    pub name: Option<String>,
    pub hull_length: Option<f64>,
    pub hull_width: Option<f64>,
    pub building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub engine_building_year: Option<u32>,
    pub owner: Option<String>,
}
