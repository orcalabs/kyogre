use crate::*;
use async_trait::async_trait;
use test_helper::{ais::AisPositionBuilder, ais_vms::AisVmsPositionBuilder};

use super::{
    ais::{AisPositionConstructor, AisPositionTripBuilder},
    ais_vms::{AisVmsPositionConstructor, AisVmsPositionTripBuilder},
    cycle::Cycle,
};

pub trait Cycleable
where
    Self: Sized,
{
    type Constructor;
    fn new_cycle(self) -> Self;
    fn cycle(constructor: Self::Constructor) -> Cycle;
}

macro_rules! impl_cycleable {
    ($type: ty, $constructor: ty, $($field_path:ident).+, $($cycle_path:ident).+) => {
        impl Cycleable for $type {
            type Constructor = $constructor;
            fn new_cycle(mut self) -> Self {
                self.$($field_path).+.cycle.increment();
                self
            }
            fn cycle(constructor: Self::Constructor) -> Cycle {
                constructor.$($cycle_path).+
            }
        }
    };
}

impl_cycleable!(WeatherBuilder, WeatherConstructor, state, cycle);
impl_cycleable!(AisVesselBuilder, AisVesselConstructor, state, cycle);
impl_cycleable!(
    OceanClimateHaulBuilder,
    OceanClimateConstructor,
    state.state.state,
    cycle
);
impl_cycleable!(
    AisVmsPositionTripBuilder,
    AisVmsPositionConstructor,
    state.state.state,
    cycle
);
impl_cycleable!(
    AisPositionTripBuilder,
    AisPositionConstructor,
    state.state.state,
    cycle
);
impl_cycleable!(
    VmsPositionTripBuilder,
    VmsPositionConstructor,
    state.state.state,
    cycle
);
impl_cycleable!(TripBuilder, TripConstructor, state.state, cycle);
impl_cycleable!(
    AisVmsPositionBuilder,
    AisVmsPositionConstructor,
    state.state,
    cycle
);
impl_cycleable!(PorVesselBuilder, PorConstructor, state.state, cycle);
impl_cycleable!(DepVesselBuilder, DepConstructor, state.state, cycle);
impl_cycleable!(TraVesselBuilder, TraConstructor, state.state, cycle);
impl_cycleable!(TraTripBuilder, TraConstructor, state.state.state, cycle);
impl_cycleable!(VesselBuilder, VesselContructor, state, cycle);
impl_cycleable!(HaulBuilder, HaulConstructor, state, cycle);
impl_cycleable!(HaulTripBuilder, HaulConstructor, state.state.state, cycle);
impl_cycleable!(HaulVesselBuilder, HaulConstructor, state.state, cycle);
impl_cycleable!(
    VmsPositionBuilder,
    VmsPositionConstructor,
    state.state,
    cycle
);
impl_cycleable!(
    AisPositionBuilder,
    AisPositionConstructor,
    state.state,
    cycle
);
impl_cycleable!(
    ManualDeliveryPointsBuilder,
    ManualDeliveryPointConstructor,
    state,
    cycle
);
impl_cycleable!(AquaCultureBuilder, AquaCultureConstructor, state, cycle);
impl_cycleable!(MattilsynetBuilder, MattilsynetConstructor, state, cycle);
impl_cycleable!(
    FishingFacilityBuilder,
    FishingFacilityConctructor,
    state,
    cycle
);
impl_cycleable!(
    FishingFacilityVesselBuilder,
    FishingFacilityConctructor,
    state.state,
    cycle
);
impl_cycleable!(
    FishingFacilityTripBuilder,
    FishingFacilityConctructor,
    state.state.state,
    cycle
);
impl_cycleable!(LandingBuilder, LandingConstructor, state, cycle);
impl_cycleable!(LandingVesselBuilder, LandingConstructor, state.state, cycle);
impl_cycleable!(
    LandingTripBuilder,
    LandingConstructor,
    state.state.state,
    cycle
);
impl_cycleable!(
    WeatherHaulBuilder,
    WeatherConstructor,
    state.state.state,
    cycle
);

pub trait Modifiable
where
    Self: Sized,
{
    type Constructor;
    fn current_index(&self) -> usize;
    fn slice(&mut self) -> &mut [Self::Constructor];
    fn modify<F>(mut self, closure: F) -> Self
    where
        F: Fn(&mut Self::Constructor),
    {
        let current_index = self.current_index();
        self.slice()
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    fn modify_idx<F>(mut self, closure: F) -> Self
    where
        F: Fn(usize, &mut Self::Constructor),
    {
        let current_index = self.current_index();
        self.slice()
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= current_index)
            .for_each(|(idx, c)| closure(idx - current_index, c));

        self
    }
}

macro_rules! impl_modifiable {
    ($type: ty, $constructor: ty, $($field_path:ident).+) => {
        impl Modifiable for $type {
            type Constructor = $constructor;
            fn current_index(&self) -> usize {
                self.current_index
            }
            fn slice(&mut self) -> &mut [Self::Constructor] {
                &mut self.$($field_path).+
            }
        }
    };
}

impl_modifiable!(
    AisVmsPositionBuilder,
    AisVmsPositionConstructor,
    state.state.ais_vms_positions
);
impl_modifiable!(AisVesselBuilder, AisVesselConstructor, state.ais_static);
impl_modifiable!(TraVesselBuilder, TraConstructor, state.state.tra);
impl_modifiable!(TraTripBuilder, TraConstructor, state.state.state.tra);
impl_modifiable!(VesselBuilder, VesselContructor, state.vessels);
impl_modifiable!(HaulBuilder, HaulConstructor, state.hauls);
impl_modifiable!(HaulTripBuilder, HaulConstructor, state.state.state.hauls);
impl_modifiable!(HaulVesselBuilder, HaulConstructor, state.state.hauls);
impl_modifiable!(
    VmsPositionBuilder,
    VmsPositionConstructor,
    state.state.vms_positions
);
impl_modifiable!(
    AisPositionBuilder,
    AisPositionConstructor,
    state.state.ais_positions
);
impl_modifiable!(
    ManualDeliveryPointsBuilder,
    ManualDeliveryPointConstructor,
    state.manual_delivery_points
);
impl_modifiable!(
    AisPositionTripBuilder,
    AisPositionConstructor,
    state.state.state.ais_positions
);
impl_modifiable!(
    AisVmsPositionTripBuilder,
    AisVmsPositionConstructor,
    state.state.state.ais_vms_positions
);
impl_modifiable!(
    VmsPositionTripBuilder,
    VmsPositionConstructor,
    state.state.state.vms_positions
);
impl_modifiable!(
    AquaCultureBuilder,
    AquaCultureConstructor,
    state.aqua_cultures
);
impl_modifiable!(
    MattilsynetBuilder,
    MattilsynetConstructor,
    state.mattilsynet
);
impl_modifiable!(DepVesselBuilder, DepConstructor, state.state.dep);
impl_modifiable!(
    FishingFacilityBuilder,
    FishingFacilityConctructor,
    state.fishing_facilities
);
impl_modifiable!(
    FishingFacilityVesselBuilder,
    FishingFacilityConctructor,
    state.state.fishing_facilities
);
impl_modifiable!(
    FishingFacilityTripBuilder,
    FishingFacilityConctructor,
    state.state.state.fishing_facilities
);
impl_modifiable!(LandingBuilder, LandingConstructor, state.landings);
impl_modifiable!(
    LandingVesselBuilder,
    LandingConstructor,
    state.state.landings
);
impl_modifiable!(
    LandingTripBuilder,
    LandingConstructor,
    state.state.state.landings
);
impl_modifiable!(PorVesselBuilder, PorConstructor, state.state.por);
impl_modifiable!(
    WeatherHaulBuilder,
    WeatherConstructor,
    state.state.state.weather
);
impl_modifiable!(WeatherBuilder, WeatherConstructor, state.weather);

impl Modifiable for TripBuilder {
    type Constructor = TripConstructor;

    fn current_index(&self) -> usize {
        self.current_index
    }

    fn slice(&mut self) -> &mut [Self::Constructor] {
        &mut self.state.state.trips
    }

    fn modify<F>(mut self, closure: F) -> Self
    where
        F: Fn(&mut Self::Constructor),
    {
        let current_index = self.current_index();
        self.slice()
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= current_index)
            .for_each(|(_, c)| {
                closure(c);
                c.current_data_timestamp = c.start();
            });

        self
    }

    fn modify_idx<F>(mut self, closure: F) -> Self
    where
        F: Fn(usize, &mut Self::Constructor),
    {
        let current_index = self.current_index();
        self.slice()
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= current_index)
            .for_each(|(idx, c)| {
                closure(idx, c);
                c.current_data_timestamp = c.start();
            });

        self
    }
}

#[async_trait]
pub trait HaulVesselLevel
where
    Self: Sized,
{
    fn up(self) -> HaulVesselBuilder;
    fn base(self) -> TestStateBuilder;
    async fn build(self) -> TestState {
        self.base().build().await
    }
}

macro_rules! impl_haul_vessel_level {
    ($type: ty) => {
        impl HaulVesselLevel for $type {
            fn up(self) -> HaulVesselBuilder {
                self.state
            }
            fn base(self) -> TestStateBuilder {
                self.state.state.state
            }
        }
    };
}

impl_haul_vessel_level!(WeatherHaulBuilder);
impl_haul_vessel_level!(OceanClimateHaulBuilder);

#[async_trait]
pub trait VesselLevel
where
    Self: Sized,
{
    fn up(self) -> VesselBuilder;
    fn base_ref(&mut self) -> &mut TestStateBuilder;
    fn base(self) -> TestStateBuilder;
    async fn build(self) -> TestState {
        self.base().build().await
    }
    fn trips(self, amount: usize) -> TripBuilder {
        self.up().trips(amount)
    }
    fn vessels(self, amount: usize) -> VesselBuilder {
        self.up().vessels(amount)
    }
    fn hauls(self, amount: usize) -> HaulVesselBuilder {
        self.up().hauls(amount)
    }
    fn tra(self, amount: usize) -> TraVesselBuilder {
        self.up().tra(amount)
    }
    fn dep(self, amount: usize) -> DepVesselBuilder {
        self.up().dep(amount)
    }
    fn por(self, amount: usize) -> PorVesselBuilder {
        self.up().por(amount)
    }
    fn landings(self, amount: usize) -> LandingVesselBuilder {
        self.up().landings(amount)
    }
    fn fishing_facilities(self, amount: usize) -> FishingFacilityVesselBuilder {
        self.up().fishing_facilities(amount)
    }
    fn ais_positions(self, amount: usize) -> AisPositionBuilder {
        self.up().ais_positions(amount)
    }
    fn ais_vms_positions(self, amount: usize) -> AisVmsPositionBuilder {
        self.up().ais_vms_positions(amount)
    }
}

macro_rules! impl_vessel_level {
    ($type: ty) => {
        impl VesselLevel for $type {
            fn up(self) -> VesselBuilder {
                self.state
            }
            fn base(self) -> TestStateBuilder {
                self.state.state
            }
            fn base_ref(&mut self) -> &mut TestStateBuilder {
                &mut self.state.state
            }
        }
    };
}

impl_vessel_level!(LandingVesselBuilder);
impl_vessel_level!(TripBuilder);
impl_vessel_level!(TraVesselBuilder);
impl_vessel_level!(DepVesselBuilder);
impl_vessel_level!(HaulVesselBuilder);
impl_vessel_level!(PorVesselBuilder);
impl_vessel_level!(VmsPositionBuilder);
impl_vessel_level!(AisPositionBuilder);
impl_vessel_level!(AisVmsPositionBuilder);
impl_vessel_level!(FishingFacilityVesselBuilder);

#[async_trait]
pub trait GlobalLevel
where
    Self: Sized,
{
    fn base(self) -> TestStateBuilder;
    async fn build(self) -> TestState {
        self.base().build().await
    }
    fn vessels(self, amount: usize) -> VesselBuilder {
        self.base().vessels(amount)
    }
    fn hauls(self, amount: usize) -> HaulBuilder {
        self.base().hauls(amount)
    }
    fn tra(self, amount: usize) -> TraBuilder {
        self.base().tra(amount)
    }
    fn landings(self, amount: usize) -> LandingBuilder {
        self.base().landings(amount)
    }
    fn mattilsynet(self, amount: usize) -> MattilsynetBuilder {
        self.base().mattilsynet(amount)
    }
    fn aqua_cultures(self, amount: usize) -> AquaCultureBuilder {
        self.base().aqua_cultures(amount)
    }
    fn fishing_facilities(self, amount: usize) -> FishingFacilityBuilder {
        self.base().fishing_facilities(amount)
    }
    fn manual_delivery_points(self, amount: usize) -> ManualDeliveryPointsBuilder {
        self.base().manual_delivery_points(amount)
    }
}

macro_rules! impl_global_level {
    ($type: ty) => {
        impl GlobalLevel for $type {
            fn base(self) -> TestStateBuilder {
                self.state
            }
        }
    };
}

impl_global_level!(AisVesselBuilder);
impl_global_level!(VesselBuilder);
impl_global_level!(HaulBuilder);
impl_global_level!(TraBuilder);
impl_global_level!(LandingBuilder);
impl_global_level!(MattilsynetBuilder);
impl_global_level!(AquaCultureBuilder);
impl_global_level!(FishingFacilityBuilder);
impl_global_level!(ManualDeliveryPointsBuilder);
impl_global_level!(WeatherBuilder);

#[async_trait]
pub trait TripLevel
where
    Self: Sized,
{
    fn base(self) -> TestStateBuilder;
    fn up(self) -> TripBuilder;
    async fn build(self) -> TestState {
        self.base().build().await
    }
    fn hauls(self, amount: usize) -> HaulTripBuilder {
        self.up().hauls(amount)
    }
    fn tra(self, amount: usize) -> TraTripBuilder {
        self.up().tra(amount)
    }
    fn landings(self, amount: usize) -> LandingTripBuilder {
        self.up().landings(amount)
    }
    fn ais_positions(self, amount: usize) -> AisPositionTripBuilder {
        self.up().ais_positions(amount)
    }
    fn vms_positions(self, amount: usize) -> VmsPositionTripBuilder {
        self.up().vms_positions(amount)
    }
    fn ais_vms_positions(self, amount: usize) -> AisVmsPositionTripBuilder {
        self.up().ais_vms_positions(amount)
    }
    fn fishing_facilities(self, amount: usize) -> FishingFacilityTripBuilder {
        self.up().fishing_facilities(amount)
    }
}

macro_rules! impl_trip_level {
    ($type: ty) => {
        impl TripLevel for $type {
            fn base(self) -> TestStateBuilder {
                self.state.state.state
            }
            fn up(self) -> TripBuilder {
                self.state
            }
        }
    };
}

impl_trip_level!(HaulTripBuilder);
impl_trip_level!(TraTripBuilder);
impl_trip_level!(LandingTripBuilder);
impl_trip_level!(FishingFacilityTripBuilder);
impl_trip_level!(AisVmsPositionTripBuilder);
impl_trip_level!(AisPositionTripBuilder);
impl_trip_level!(VmsPositionTripBuilder);
