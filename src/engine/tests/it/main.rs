#![deny(warnings)]
#![deny(rust_2018_idioms)]

use config::{Config, File};
use engine::Settings;

pub mod ers_assembler;
pub mod haul_distributor;
pub mod helper;
pub mod landings_assembler;
pub mod trip_distance;
pub mod trips;
pub mod trips_precision;

#[test]
fn test_local_settings_are_valid() {
    Config::builder()
        .add_source(File::with_name("config/local.yml").required(true))
        .set_override("postgres.username", "test")
        .unwrap()
        .set_override("environment", "Local")
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize::<Settings>()
        .unwrap();
}

#[test]
fn test_development_settings_are_valid() {
    Config::builder()
        .add_source(File::with_name("config/development.yml").required(true))
        .set_override("postgres.username", "test")
        .unwrap()
        .set_override("environment", "Development")
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize::<Settings>()
        .unwrap();
}
