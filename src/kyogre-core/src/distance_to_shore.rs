use flate2::read::GzDecoder;
use geo::{point, prelude::*};
use orca_core::Environment;
use serde::Deserialize;
use std::{io::prelude::*, sync::OnceLock};

static SHORELINE: OnceLock<vpsearch::Tree<Coordinate>> = OnceLock::new();

#[derive(Copy, Clone, Debug, Deserialize)]
struct Coordinate {
    longitude: f64,
    latitude: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct Coordinates {
    coordinates: Vec<Coordinate>,
}

pub fn distance_to_shore(latitude: f64, longitude: f64) -> f64 {
    // Avoid unzipping shoreline map for each test, takes a significant amount of time.
    if running_in_test() {
        0.0
    } else {
        let (_, val) = SHORELINE
            .get_or_init(create_vp_tree)
            .find_nearest(&Coordinate {
                longitude,
                latitude,
            });

        val
    }
}

fn haversine_distance(point1: &Coordinate, point2: &Coordinate) -> f64 {
    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);
    p1.haversine_distance(&p2)
}

fn decompress_shoreline() -> String {
    let bytes = include_bytes!("../shoreline.json.gz");
    let mut d = GzDecoder::new(&bytes[..]);
    let mut s = String::new();
    d.read_to_string(&mut s).unwrap();
    s
}

fn create_vp_tree() -> vpsearch::Tree<Coordinate> {
    let data = decompress_shoreline();
    let data: Coordinates = serde_json::from_str(&data).unwrap();
    vpsearch::Tree::new(&data.coordinates)
}

impl vpsearch::MetricSpace for Coordinate {
    type UserData = ();
    type Distance = f64;
    fn distance(&self, other: &Self, _: &Self::UserData) -> Self::Distance {
        haversine_distance(self, other)
    }
}

pub fn running_in_test() -> bool {
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or("Test".into())
        .try_into()
        .unwrap();

    environment == Environment::Test
}
