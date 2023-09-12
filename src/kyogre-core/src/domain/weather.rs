use chrono::{DateTime, Utc};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct NewWeather {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Weather {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
    pub weather_location_id: i32,
}

#[derive(Debug, Clone)]
pub struct HaulWeather {
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
}

static LATS_LONS: [(f64, f64); 20] = [
    (52.380, 1.8924),
    (52.352, 1.9485),
    (52.357, 2.0484),
    (52.363, 2.1499),
    (52.369, 2.2486),
    (52.376, 2.3474),
    (52.381, 2.4476),
    (52.387, 2.5453),
    (52.392, 2.6376),
    (52.399, 2.7079),
    (52.458, 1.8801),
    (52.448, 1.9519),
    (52.450, 2.0505),
    (52.450, 2.1503),
    (52.449, 2.2514),
    (52.450, 2.3504),
    (52.450, 2.4490),
    (52.449, 2.5493),
    (52.449, 2.6498),
    (52.450, 2.7498),
];

impl NewWeather {
    pub fn test_default(timestamp: DateTime<Utc>) -> Self {
        let mut rng = rand::thread_rng();
        let (latitude, longitude) = LATS_LONS[rng.gen::<usize>() % LATS_LONS.len()];
        let num: f64 = rng.gen();

        Self {
            timestamp,
            latitude,
            longitude,
            altitude: num,
            wind_speed_10m: Some(num),
            wind_direction_10m: Some(num),
            air_temperature_2m: Some(num),
            relative_humidity_2m: Some(num),
            air_pressure_at_sea_level: Some(num),
            precipitation_amount: Some(num),
            land_area_fraction: 0.,
            cloud_area_fraction: Some(0.),
        }
    }
}
