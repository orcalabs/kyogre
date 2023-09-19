use crate::*;

use super::cycle::Cycle;

pub struct WeatherHaulBuilder {
    pub state: HaulVesselBuilder,
    pub current_index: usize,
}

pub struct WeatherConstructor {
    pub weather: NewWeather,
    pub cycle: Cycle,
}
