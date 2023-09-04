use fiskeridir_rs::ErsDca;

use crate::{
    FishingFacilityTripBuilder, LandingBuilder, LandingVesselBuilder, TestState, TestStateBuilder,
    TraBuilder, TraTripBuilder, TraVesselBuilder, VesselBuilder,
};

use super::{landing::LandingTripBuilder, trip::TripBuilder};

pub struct HaulBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct HaulTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct HaulVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct HaulConstructor {
    pub dca: ErsDca,
}

impl HaulBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub fn landings(self, amount: usize) -> LandingBuilder {
        self.state.landings(amount)
    }
    pub fn tra(self, amount: usize) -> TraBuilder {
        self.state.tra(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> HaulBuilder
    where
        F: Fn(&mut HaulConstructor),
    {
        self.state
            .hauls
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> HaulBuilder
    where
        F: Fn(usize, &mut HaulConstructor),
    {
        self.state
            .hauls
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl HaulVesselBuilder {
    pub fn trips(self, amount: usize) -> TripBuilder {
        self.state.trips(amount)
    }
    pub fn landings(self, amount: usize) -> LandingVesselBuilder {
        self.state.landings(amount)
    }
    pub fn tra(self, amount: usize) -> TraVesselBuilder {
        self.state.tra(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> HaulVesselBuilder
    where
        F: Fn(&mut HaulConstructor),
    {
        self.state
            .state
            .hauls
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> HaulVesselBuilder
    where
        F: Fn(usize, &mut HaulConstructor),
    {
        self.state
            .state
            .hauls
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
impl HaulTripBuilder {
    pub fn landings(self, amount: usize) -> LandingTripBuilder {
        self.state.landings(amount)
    }
    pub fn tra(self, amount: usize) -> TraTripBuilder {
        self.state.tra(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.state.state.build().await
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityTripBuilder {
        self.state.fishing_facilities(amount)
    }
    pub fn modify<F>(mut self, closure: F) -> HaulTripBuilder
    where
        F: Fn(&mut HaulConstructor),
    {
        self.state
            .state
            .state
            .hauls
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> HaulTripBuilder
    where
        F: Fn(usize, &mut HaulConstructor),
    {
        self.state
            .state
            .state
            .hauls
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
