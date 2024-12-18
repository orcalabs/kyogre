use kyogre_core::WeeklySale;

use super::{cycle::Cycle, LandingBuilder, LandingTripBuilder, TestStateBuilder};

pub struct WeeklySaleBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct WeeklySaleLandingBuilder {
    pub state: LandingBuilder,
    pub current_index: usize,
}

pub struct WeeklySaleLandingTripBuilder {
    pub state: LandingTripBuilder,
    pub current_index: usize,
}

pub struct WeeklySaleContructor {
    pub cycle: Cycle,
    pub weekly_sale: WeeklySale,
}
