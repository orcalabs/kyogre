use crate::*;

use super::cycle::Cycle;

pub struct WeatherBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct WeatherHaulBuilder {
    pub state: HaulVesselBuilder,
    pub current_index: usize,
}

pub struct WeatherConstructor {
    pub weather: NewWeather,
    pub cycle: Cycle,
}
