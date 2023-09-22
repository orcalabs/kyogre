use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{angle_between_vectors, length_of_vector};

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct OceanClimate {
    pub lat: f64,
    pub lon: f64,
    pub u: Option<f64>,
    pub v: Option<f64>,
    pub w: Option<f64>,
    pub Uwind: Option<f64>,
    pub Vwind: Option<f64>,
    pub salinity: Option<f64>,
    pub temperature: Option<f64>,
    pub h: f64,
}

impl OceanClimate {
    pub fn to_core_ocean_climate(
        v: OceanClimate,
        timestamp: DateTime<Utc>,
        depth: i32,
    ) -> kyogre_core::NewOceanClimate {
        let (water_speed, water_direction) = if let (Some(x), Some(y)) = (v.u, v.v) {
            (
                Some(length_of_vector((x, y))),
                Some(angle_between_vectors((1., 0.), (x, y))),
            )
        } else {
            (None, None)
        };

        let (wind_speed, wind_direction) = if let (Some(x), Some(y)) = (v.Uwind, v.Vwind) {
            (
                Some(length_of_vector((x, y))),
                Some(angle_between_vectors((1., 0.), (x, y))),
            )
        } else {
            (None, None)
        };

        kyogre_core::NewOceanClimate {
            timestamp,
            depth,
            latitude: v.lat,
            longitude: v.lon,
            water_speed,
            water_direction,
            upward_sea_velocity: v.w,
            wind_speed,
            wind_direction,
            salinity: v.salinity,
            temperature: v.temperature,
            sea_floor_depth: v.h,
        }
    }
}
