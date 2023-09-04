use crate::*;

pub struct FishingFacilityTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityConctructor {
    pub facility: FishingFacility,
}

impl FishingFacilityTripBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state.state
    }
    pub fn landings(self, amount: usize) -> LandingTripBuilder {
        self.state.landings(amount)
    }
    pub fn tra(self, amount: usize) -> TraTripBuilder {
        self.state.tra(amount)
    }
    pub fn hauls(self, amount: usize) -> HaulTripBuilder {
        self.state.hauls(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> FishingFacilityTripBuilder
    where
        F: Fn(&mut FishingFacilityConctructor),
    {
        self.state
            .state
            .state
            .fishing_facilities
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> FishingFacilityTripBuilder
    where
        F: Fn(usize, &mut FishingFacilityConctructor),
    {
        self.state
            .state
            .state
            .fishing_facilities
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
