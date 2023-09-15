use crate::*;
use async_trait::async_trait;
use test_helper::{ais::AisPositionBuilder, ais_vms::AisVmsPositionBuilder};

use super::ais::AisPositionConstructor;

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
            .for_each(|(idx, c)| closure(idx, c));

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
impl_modifiable!(LandingBuilder, fiskeridir_rs::Landing, state.landings);
impl_modifiable!(
    LandingVesselBuilder,
    fiskeridir_rs::Landing,
    state.state.landings
);
impl_modifiable!(
    LandingTripBuilder,
    fiskeridir_rs::Landing,
    state.state.state.landings
);
impl_modifiable!(PorVesselBuilder, PorConstructor, state.state.por);

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
pub trait VesselLevel
where
    Self: Sized,
{
    fn up(self) -> VesselBuilder;
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

impl_global_level!(VesselBuilder);
impl_global_level!(HaulBuilder);
impl_global_level!(TraBuilder);
impl_global_level!(LandingBuilder);
impl_global_level!(MattilsynetBuilder);
impl_global_level!(AquaCultureBuilder);
impl_global_level!(FishingFacilityBuilder);
impl_global_level!(ManualDeliveryPointsBuilder);

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
    fn fishing_facilities(self, amount: usize) -> FishingFacilityTripBuilder {
        self.up().fishing_facilities(amount)
    }
}

macro_rules! impl_global_level {
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

impl_global_level!(HaulTripBuilder);
impl_global_level!(TraTripBuilder);
impl_global_level!(LandingTripBuilder);
impl_global_level!(FishingFacilityTripBuilder);
