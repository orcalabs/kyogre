use chrono::{DateTime, NaiveDate, Utc};
use geo::geometry::Polygon;
use serde::{Deserialize, Serialize};

use crate::{CatchLocationId, HaulId, HaulOceanClimate, HaulWeatherStatus};

static MAX_WIND_SPEED_10M: f64 = 1000.0;
static MIN_WIND_SPEED_10M: f64 = 0.0;

static MAX_AIR_TEMPERATURE_2M: f64 = 1000.0;
static MIN_AIR_TEMPERATURE_2M: f64 = -1000.0;

static MAX_RELATIVE_HUMIDITY_2M: f64 = 1.0;
static MIN_RELATIVE_HUMIDITY_2M: f64 = 0.0;

static MAX_AIR_PRESSURE_AT_SEA_LEVEL: f64 = 300000.0;
static MIN_AIR_PRESSURE_AT_SEA_LEVEL: f64 = 10000.0;

static MAX_PRECIPITATION_AMOUNT: f64 = 1000.0;
static MIN_PRECIPITATION_AMOUNT: f64 = -1000.0;

static MAX_CLOUD_AREA_FRACTION: f64 = 1.0;
static MIN_CLOUD_AREA_FRACTION: f64 = 0.0;

#[derive(Copy, Clone, Debug)]
pub enum WeatherData {
    Require,
    Optional,
}

#[derive(Default, Clone, Debug, Copy)]
pub struct WindSpeed(Option<f64>);
impl WindSpeed {
    pub fn into_inner(self) -> Option<f64> {
        self.0
    }
    pub fn new(val: f64) -> WindSpeed {
        if val < MAX_WIND_SPEED_10M || val > MIN_WIND_SPEED_10M {
            WindSpeed(Some(val))
        } else {
            WindSpeed(None)
        }
    }
}

#[derive(Default, Clone, Debug, Copy)]
pub struct AirTemperature(Option<f64>);
impl AirTemperature {
    pub fn into_inner(self) -> Option<f64> {
        self.0
    }
    pub fn new(val: f64) -> AirTemperature {
        if val < MAX_AIR_TEMPERATURE_2M || val > MIN_AIR_TEMPERATURE_2M {
            AirTemperature(Some(val))
        } else {
            AirTemperature(None)
        }
    }
}

#[derive(Default, Clone, Debug, Copy)]
pub struct RelativeHumidity(Option<f64>);
impl RelativeHumidity {
    pub fn into_inner(self) -> Option<f64> {
        self.0
    }
    pub fn new(val: f64) -> RelativeHumidity {
        if val < MAX_RELATIVE_HUMIDITY_2M || val > MIN_RELATIVE_HUMIDITY_2M {
            RelativeHumidity(Some(val))
        } else {
            RelativeHumidity(None)
        }
    }
}

#[derive(Default, Clone, Debug, Copy)]
pub struct AirPressureAtSeaLevel(Option<f64>);
impl AirPressureAtSeaLevel {
    pub fn into_inner(self) -> Option<f64> {
        self.0
    }
    pub fn new(val: f64) -> AirPressureAtSeaLevel {
        if val < MAX_AIR_PRESSURE_AT_SEA_LEVEL || val > MIN_AIR_PRESSURE_AT_SEA_LEVEL {
            AirPressureAtSeaLevel(Some(val))
        } else {
            AirPressureAtSeaLevel(None)
        }
    }
}

#[derive(Default, Clone, Debug, Copy)]
pub struct PrecipitationAmount(Option<f64>);
impl PrecipitationAmount {
    pub fn into_inner(self) -> Option<f64> {
        self.0
    }
    pub fn new(val: f64) -> PrecipitationAmount {
        if val < MAX_PRECIPITATION_AMOUNT || val > MIN_PRECIPITATION_AMOUNT {
            PrecipitationAmount(Some(val))
        } else {
            PrecipitationAmount(None)
        }
    }
}

#[derive(Default, Clone, Debug, Copy)]
pub struct CloudAreaFraction(Option<f64>);
impl CloudAreaFraction {
    pub fn into_inner(self) -> Option<f64> {
        self.0
    }
    pub fn new(val: f64) -> CloudAreaFraction {
        if val < MAX_CLOUD_AREA_FRACTION || val > MIN_CLOUD_AREA_FRACTION {
            CloudAreaFraction(Some(val))
        } else {
            CloudAreaFraction(None)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatchLocationWeather {
    #[serde(skip_serializing)]
    pub id: CatchLocationId,
    pub date: NaiveDate,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: f64,
    pub air_temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub air_pressure_at_sea_level: f64,
    pub precipitation_amount: f64,
    pub cloud_area_fraction: f64,
}

#[derive(Debug, Clone)]
pub struct NewWeather {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: WindSpeed,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: AirTemperature,
    pub relative_humidity_2m: RelativeHumidity,
    pub air_pressure_at_sea_level: AirPressureAtSeaLevel,
    pub precipitation_amount: PrecipitationAmount,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: CloudAreaFraction,
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
    pub weather_location_id: WeatherLocationId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct WeatherLocationId(pub i32);

#[derive(Debug, Clone)]
pub struct WeatherLocation {
    pub id: WeatherLocationId,
    pub polygon: Polygon,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HaulWeather {
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct HaulWeatherOutput {
    pub haul_id: HaulId,
    pub weather: Option<HaulWeather>,
    pub ocean_climate: Option<HaulOceanClimate>,
    pub status: HaulWeatherStatus,
}

pub static WEATHER_LOCATION_LATS_LONS: [(f64, f64, i64); 21] = [
    (71.35, 35.95, 171301359),
    (71.05, 28.05, 171001280),
    (71.05, 27.95, 171001279),
    (71.05, 25.95, 171001259),
    (71.04, 25.85, 171001258),
    (71.05, 25.75, 171001257),
    (71.04, 25.65, 171001256),
    (71.05, 25.55, 171001255),
    (70.95, 28.35, 170901283),
    (70.95, 28.24, 170901282),
    (70.95, 28.15, 170901281),
    (70.95, 28.05, 170901280),
    (70.95, 27.95, 170901279),
    (70.95, 27.84, 170901278),
    (70.95, 27.55, 170901275),
    (70.95, 27.44, 170901274),
    (70.95, 27.34, 170901273),
    (70.95, 26.65, 170901266),
    (70.95, 26.55, 170901265),
    (70.95, 25.55, 170901255),
    (70.95, 25.45, 170901254),
];

impl NewWeather {
    pub fn test_default(timestamp: DateTime<Utc>) -> Self {
        let (latitude, longitude, _) = WEATHER_LOCATION_LATS_LONS[0];
        Self {
            timestamp,
            latitude,
            longitude,
            altitude: 0.0,
            wind_speed_10m: WindSpeed::new(20.0),
            wind_direction_10m: Some(150.0),
            air_temperature_2m: AirTemperature::new(10.0),
            relative_humidity_2m: RelativeHumidity::new(0.2),
            air_pressure_at_sea_level: AirPressureAtSeaLevel::new(20000.0),
            precipitation_amount: PrecipitationAmount::new(10.0),
            land_area_fraction: 0.,
            cloud_area_fraction: CloudAreaFraction::new(0.2),
        }
    }
}

impl From<&NewWeather> for Weather {
    fn from(v: &NewWeather) -> Self {
        Self {
            timestamp: v.timestamp,
            latitude: v.latitude,
            longitude: v.longitude,
            altitude: v.altitude,
            wind_speed_10m: v.wind_speed_10m.into_inner(),
            wind_direction_10m: v.wind_direction_10m,
            air_temperature_2m: v.air_temperature_2m.into_inner(),
            relative_humidity_2m: v.relative_humidity_2m.into_inner(),
            air_pressure_at_sea_level: v.air_pressure_at_sea_level.into_inner(),
            precipitation_amount: v.precipitation_amount.into_inner(),
            land_area_fraction: v.land_area_fraction,
            cloud_area_fraction: v.cloud_area_fraction.into_inner(),
            weather_location_id: WeatherLocationId::from_lat_lon(v.latitude, v.longitude),
        }
    }
}

impl WeatherLocationId {
    pub fn from_lat_lon(lat: f64, lon: f64) -> Self {
        Self(
            (((lat / 0.1).floor() as i32 + 1_000) * 100_000) + ((lon / 0.1).floor() as i32 + 1_000),
        )
    }
}

impl PartialEq for WeatherLocation {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WeatherLocation {}

impl std::hash::Hash for WeatherLocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<Option<f64>> for WindSpeed {
    fn from(value: Option<f64>) -> Self {
        value.map(WindSpeed::new).unwrap_or_default()
    }
}

impl From<Option<f64>> for AirTemperature {
    fn from(value: Option<f64>) -> Self {
        value.map(AirTemperature::new).unwrap_or_default()
    }
}

impl From<Option<f64>> for RelativeHumidity {
    fn from(value: Option<f64>) -> Self {
        value.map(RelativeHumidity::new).unwrap_or_default()
    }
}

impl From<Option<f64>> for AirPressureAtSeaLevel {
    fn from(value: Option<f64>) -> Self {
        value.map(AirPressureAtSeaLevel::new).unwrap_or_default()
    }
}

impl From<Option<f64>> for CloudAreaFraction {
    fn from(value: Option<f64>) -> Self {
        value.map(CloudAreaFraction::new).unwrap_or_default()
    }
}

impl From<Option<f64>> for PrecipitationAmount {
    fn from(value: Option<f64>) -> Self {
        value.map(PrecipitationAmount::new).unwrap_or_default()
    }
}
