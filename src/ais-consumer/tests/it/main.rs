#![deny(warnings)]
#![deny(rust_2018_idioms)]

use ais_consumer::settings::Settings;
use config::{Config, File};

pub mod consumer;
pub mod helper;

#[test]
fn test_local_settings_are_valid() {
    Config::builder()
        .add_source(File::with_name("config/local.yml").required(true))
        .set_override("postgres.username", "test")
        .unwrap()
        .set_override("environment", "Local")
        .unwrap()
        .set_override("oauth.client_id", "test")
        .unwrap()
        .set_override("oauth.client_secret", "test")
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
        .set_override("oauth.client_id", "test")
        .unwrap()
        .set_override("oauth.client_secret", "test")
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize::<Settings>()
        .unwrap();
}
