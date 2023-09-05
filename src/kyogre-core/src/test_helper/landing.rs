use crate::*;

pub struct LandingBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct LandingVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct LandingTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

impl LandingTripBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state.state
    }
    pub fn up(self) -> VesselBuilder {
        self.state.state
    }
    pub fn hauls(self, amount: usize) -> HaulTripBuilder {
        self.state.hauls(amount)
    }
    pub fn tra(self, amount: usize) -> TraTripBuilder {
        self.state.tra(amount)
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityTripBuilder {
        self.state.fishing_facilities(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.state.state.build().await
    }

    pub fn modify<F>(mut self, closure: F) -> LandingTripBuilder
    where
        F: Fn(&mut fiskeridir_rs::Landing),
    {
        self.state
            .state
            .state
            .landings
            .iter_mut()
            .enumerate()
            .for_each(|(i, landing)| {
                if i >= self.current_index {
                    closure(landing)
                }
            });
        self
    }
    pub fn modify_idx<F>(mut self, closure: F) -> LandingTripBuilder
    where
        F: Fn(usize, &mut fiskeridir_rs::Landing),
    {
        self.state
            .state
            .state
            .landings
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl LandingBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }

    pub fn tra(self, amount: usize) -> TraBuilder {
        self.state.tra(amount)
    }

    pub fn hauls(self, amount: usize) -> HaulBuilder {
        self.state.hauls(amount)
    }

    pub fn modify<F>(mut self, closure: F) -> LandingBuilder
    where
        F: Fn(&mut fiskeridir_rs::Landing),
    {
        self.state
            .landings
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> LandingBuilder
    where
        F: Fn(usize, &mut fiskeridir_rs::Landing),
    {
        self.state
            .landings
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl LandingVesselBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state
    }
    pub async fn build(self) -> TestState {
        self.state.state.build().await
    }
    pub fn trips(self, amount: usize) -> TripBuilder {
        self.state.trips(amount)
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
    pub fn por(self, amount: usize) -> PorVesselBuilder {
        self.state.por(amount)
    }
    pub fn fishing_facilities(self, amount: usize) -> FishingFacilityVesselBuilder {
        self.state.fishing_facilities(amount)
    }

    pub fn modify<F>(mut self, closure: F) -> LandingVesselBuilder
    where
        F: Fn(&mut fiskeridir_rs::Landing),
    {
        self.state
            .state
            .landings
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> LandingVesselBuilder
    where
        F: Fn(usize, &mut fiskeridir_rs::Landing),
    {
        self.state
            .state
            .landings
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
