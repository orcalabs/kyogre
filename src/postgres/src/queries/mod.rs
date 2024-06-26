use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

pub mod ais;
pub mod ais_vms;
pub mod catch_area;
pub mod catch_location;
pub mod delivery_point;
pub mod duckdb;
pub mod economic_zone;
pub mod ers_common;
pub mod ers_dca;
pub mod ers_dep;
pub mod ers_por;
pub mod ers_tra;
pub mod fishing_facility;
pub mod fuel;
pub mod gear;
pub mod hash;
pub mod haul;
pub mod landing;
pub mod landing_matrix;
pub mod ml_models;
pub mod norwegian_land;
pub mod ocean_climate;
pub mod port;
pub mod species;
pub mod trip;
pub mod user;
pub mod verify_database;
pub mod vessel;
pub mod vessel_benchmarks;
pub mod vessel_events;
pub mod vms;
pub mod weather;

pub(crate) fn timestamp_from_date_and_time(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    DateTime::from_naive_utc_and_offset(date.and_time(time), Utc)
}

pub(crate) fn opt_timestamp_from_date_and_time(
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
) -> Option<DateTime<Utc>> {
    match (date, time) {
        (Some(date), Some(time)) => Some(timestamp_from_date_and_time(date, time)),
        _ => None,
    }
}

pub fn enum_to_i32<T: Into<i32>>(value: T) -> i32 {
    value.into()
}

pub fn opt_enum_to_i32<T: Into<i32>>(value: Option<T>) -> Option<i32> {
    value.map(enum_to_i32)
}
