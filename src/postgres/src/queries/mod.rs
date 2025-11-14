pub mod ais;
pub mod ais_vms;
pub mod assert;
pub mod catch_location;
pub mod current_position;
pub mod delivery_point;
pub mod duckdb;
pub mod ers_dca;
pub mod ers_dep;
pub mod ers_por;
pub mod ers_tra;
pub mod fishing_facility;
pub mod fuel;
pub mod fuel_measurement;
pub mod hash;
pub mod haul;
pub mod landing;
pub mod landing_matrix;
pub mod ocean_climate;
pub mod org;
pub mod port;
pub mod processor;
pub mod rafisklaget;
pub mod species;
#[cfg(feature = "test")]
pub mod test;
pub mod trip;
pub mod trip_benchmarks;
pub mod unnest_insert;
pub mod unnest_update;
pub mod user;
pub mod verify_database;
pub mod vessel;
pub mod vessel_benchmarks;
pub mod vessel_events;
pub mod vms;
pub mod weather;

pub fn opt_type_to_f64<T: Into<f64>>(value: Option<T>) -> Option<f64> {
    value.map(|v| v.into())
}

pub fn type_to_i32<T: Into<i32>>(value: T) -> i32 {
    value.into()
}

pub fn opt_type_to_i32<T: Into<i32>>(value: Option<T>) -> Option<i32> {
    value.map(type_to_i32)
}

pub fn type_to_i64<T: Into<i64>>(value: T) -> i64 {
    value.into()
}

pub fn opt_type_to_i64<T: Into<i64>>(value: Option<T>) -> Option<i64> {
    value.map(type_to_i64)
}

pub fn opt_type_as_static_str<T: Into<&'static str>>(value: Option<T>) -> Option<&'static str> {
    value.map(Into::into)
}
