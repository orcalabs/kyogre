use crate::*;

pub struct FishingFacilityBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct FishingFacilityConctructor {
    pub facility: FishingFacility,
}

impl FishingFacilityBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> FishingFacilityBuilder
    where
        F: Fn(&mut FishingFacilityConctructor),
    {
        self.state
            .fishing_facilities
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> FishingFacilityBuilder
    where
        F: Fn(usize, &mut FishingFacilityConctructor),
    {
        self.state
            .fishing_facilities
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl FishingFacilityVesselBuilder {
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
    pub fn por(self, amount: usize) -> PorVesselBuilder {
        self.state.por(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> FishingFacilityVesselBuilder
    where
        F: Fn(&mut FishingFacilityConctructor),
    {
        self.state
            .state
            .fishing_facilities
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> FishingFacilityVesselBuilder
    where
        F: Fn(usize, &mut FishingFacilityConctructor),
    {
        self.state
            .state
            .fishing_facilities
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
impl FishingFacilityTripBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state.state.state
    }
    pub fn up(self) -> VesselBuilder {
        self.state.state
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
