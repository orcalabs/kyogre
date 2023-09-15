use crate::*;

pub struct WeatherHaulBuilder {
    pub state: HaulVesselBuilder,
    pub current_index: usize,
}

pub struct WeatherConstructor {
    pub weather: NewWeather,
}
