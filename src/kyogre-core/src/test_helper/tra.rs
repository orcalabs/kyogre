use super::{landing::LandingTripBuilder, trip::TripBuilder};
use crate::*;
use fiskeridir_rs::ErsTra;

pub struct TraBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct TraVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct TraTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

#[derive(Clone, Debug)]
pub struct TraConstructor {
    pub tra: ErsTra,
}

impl TraBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn landings(self, amount: usize) -> LandingBuilder {
        self.state.landings(amount)
    }
    pub fn hauls(self, amount: usize) -> HaulBuilder {
        self.state.hauls(amount)
    }
    pub fn modify<F>(mut self, closure: F) -> TraBuilder
    where
        F: Fn(&mut TraConstructor),
    {
        self.state
            .tra
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> TraBuilder
    where
        F: Fn(usize, &mut TraConstructor),
    {
        self.state
            .tra
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
impl TraVesselBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityVesselBuilder {
        self.state.fishing_facilities(amount)
    }
    pub fn dep(self, amount: usize) -> DepVesselBuilder {
        self.state.dep(amount)
    }
    pub fn por(self, amount: usize) -> PorVesselBuilder {
        self.state.por(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.state.build().await
    }
    pub fn trips(self, amount: usize) -> TripBuilder {
        self.state.trips(amount)
    }
    pub fn landings(self, amount: usize) -> LandingVesselBuilder {
        self.state.landings(amount)
    }
    pub fn hauls(self, amount: usize) -> HaulVesselBuilder {
        self.state.hauls(amount)
    }
    pub fn modify<F>(mut self, closure: F) -> TraVesselBuilder
    where
        F: Fn(&mut TraConstructor),
    {
        self.state
            .state
            .tra
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> TraVesselBuilder
    where
        F: Fn(usize, &mut TraConstructor),
    {
        self.state
            .state
            .tra
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl TraTripBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state.state
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityTripBuilder {
        self.state.fishing_facilities(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.state.state.build().await
    }
    pub fn landings(self, amount: usize) -> LandingTripBuilder {
        self.state.landings(amount)
    }
    pub fn hauls(self, amount: usize) -> HaulTripBuilder {
        self.state.hauls(amount)
    }
    pub fn up(self) -> VesselBuilder {
        self.state.state
    }
    pub fn modify<F>(mut self, closure: F) -> TraTripBuilder
    where
        F: Fn(&mut TraConstructor),
    {
        self.state
            .state
            .state
            .tra
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> TraTripBuilder
    where
        F: Fn(usize, &mut TraConstructor),
    {
        self.state
            .state
            .state
            .tra
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
