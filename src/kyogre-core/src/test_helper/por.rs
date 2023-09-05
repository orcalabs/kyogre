use fiskeridir_rs::ErsPor;

use crate::*;

pub struct PorVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct PorConstructor {
    pub por: ErsPor,
}

impl PorVesselBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state
    }
    pub fn trips(self, amount: usize) -> TripBuilder {
        self.state.trips(amount)
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
    pub fn dep(self, amount: usize) -> DepVesselBuilder {
        self.state.dep(amount)
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityVesselBuilder {
        self.state.fishing_facilities(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> PorVesselBuilder
    where
        F: Fn(&mut PorConstructor),
    {
        self.state
            .state
            .por
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> PorVesselBuilder
    where
        F: Fn(usize, &mut PorConstructor),
    {
        self.state
            .state
            .por
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
