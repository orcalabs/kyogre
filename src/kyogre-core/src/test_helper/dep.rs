use fiskeridir_rs::ErsDep;

use crate::*;

pub struct DepVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct DepConstructor {
    pub dep: ErsDep,
}

impl DepVesselBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state
    }
    pub fn trips(self, amount: usize) -> TripBuilder {
        self.state.trips(amount)
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityVesselBuilder {
        self.state.fishing_facilities(amount)
    }
    pub fn landings(self, amount: usize) -> LandingVesselBuilder {
        self.state.landings(amount)
    }
    pub fn tra(self, amount: usize) -> TraVesselBuilder {
        self.state.tra(amount)
    }
    pub fn hauls(self, amount: usize) -> HaulVesselBuilder {
        self.state.hauls(amount)
    }
    pub fn por(self, amount: usize) -> PorVesselBuilder {
        self.state.por(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> DepVesselBuilder
    where
        F: Fn(&mut DepConstructor),
    {
        self.state
            .state
            .dep
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> DepVesselBuilder
    where
        F: Fn(usize, &mut DepConstructor),
    {
        self.state
            .state
            .dep
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
