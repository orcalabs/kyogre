use chrono::{DateTime, Utc};
use unnest_insert::UnnestInsert;

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "ocean_climate",
    conflict = "timestamp,depth,weather_location_id"
)]
pub struct NewOceanClimate {
    pub timestamp: DateTime<Utc>,
    pub depth: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub upward_sea_velocity: Option<f64>,
    pub wind_speed: Option<f64>,
    pub wind_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub temperature: Option<f64>,
    pub sea_floor_depth: f64,
}

impl From<kyogre_core::NewOceanClimate> for NewOceanClimate {
    fn from(v: kyogre_core::NewOceanClimate) -> Self {
        Self {
            timestamp: v.timestamp,
            depth: v.depth,
            latitude: v.latitude,
            longitude: v.longitude,
            water_speed: v.water_speed,
            water_direction: v.water_direction,
            upward_sea_velocity: v.upward_sea_velocity,
            wind_speed: v.wind_speed,
            wind_direction: v.wind_direction,
            salinity: v.salinity,
            temperature: v.temperature,
            sea_floor_depth: v.sea_floor_depth,
        }
    }
}
