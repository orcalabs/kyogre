use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

use crate::WhaleGender;

#[derive(Debug, Clone)]
#[remain::sorted]
pub struct Haul {
    pub catches: Vec<HaulCatch>,
    pub duration: i32,
    pub ers_activity_id: String,
    pub fiskeridir_vessel_id: Option<i64>,
    pub gear_fiskeridir_id: Option<i32>,
    pub haul_distance: Option<i32>,
    pub location_end_code: Option<i32>,
    pub location_start_code: Option<i32>,
    pub main_area_end_id: Option<i32>,
    pub main_area_start_id: Option<i32>,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_date: NaiveDate,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub start_time: NaiveTime,
    pub start_timestamp: DateTime<Utc>,
    pub stop_date: NaiveDate,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub stop_time: NaiveTime,
    pub stop_timestamp: DateTime<Utc>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub whale_catches: Vec<WhaleCatch>,
}

#[derive(Debug, Clone)]
#[remain::sorted]
pub struct HaulCatch {
    pub living_weight: i32,
    pub main_species_fao_id: String,
    pub main_species_fiskeridir_id: Option<i32>,
    pub species_fao_id: String,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: Option<i32>,
    pub species_main_group_id: Option<i32>,
}

#[derive(Debug, Clone)]
#[remain::sorted]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    pub gender_id: Option<WhaleGender>,
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}
