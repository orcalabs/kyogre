use fiskeridir_rs::AquaCultureEntry;

use crate::*;

pub struct ManualDeliveryPointsBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct ManualDeliveryPointConstructor {
    pub val: ManualDeliveryPoint,
}

pub struct MattilsynetBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct MattilsynetConstructor {
    pub val: MattilsynetDeliveryPoint,
}

pub struct AquaCultureBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct AquaCultureConstructor {
    pub val: AquaCultureEntry,
}

impl ManualDeliveryPointsBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub fn mattilsynet(self, amount: usize) -> MattilsynetBuilder {
        self.state.mattilsynet(amount)
    }
    pub fn aqua_cultures(self, amount: usize) -> AquaCultureBuilder {
        self.state.aqua_cultures(amount)
    }
    pub fn vessels(self, amount: usize) -> VesselBuilder {
        self.state.vessels(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }

    pub fn modify<F>(mut self, closure: F) -> ManualDeliveryPointsBuilder
    where
        F: Fn(&mut ManualDeliveryPointConstructor),
    {
        self.state
            .manual_delivery_points
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> ManualDeliveryPointsBuilder
    where
        F: Fn(usize, &mut ManualDeliveryPointConstructor),
    {
        self.state
            .manual_delivery_points
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl AquaCultureBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub fn manual_delivery_points(self, amount: usize) -> ManualDeliveryPointsBuilder {
        self.state.manual_delivery_points(amount)
    }
    pub fn mattilsynet(self, amount: usize) -> MattilsynetBuilder {
        self.state.mattilsynet(amount)
    }
    pub fn vessels(self, amount: usize) -> VesselBuilder {
        self.state.vessels(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub async fn persist(mut self) -> TestStateBuilder {
        self.state
            .storage
            .add_aqua_culture_register(self.state.aqua_cultures.drain(..).map(|v| v.val).collect())
            .await
            .unwrap();

        self.base()
    }
    pub fn modify<F>(mut self, closure: F) -> AquaCultureBuilder
    where
        F: Fn(&mut AquaCultureConstructor),
    {
        self.state
            .aqua_cultures
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> AquaCultureBuilder
    where
        F: Fn(usize, &mut AquaCultureConstructor),
    {
        self.state
            .aqua_cultures
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}

impl MattilsynetBuilder {
    pub fn base(self) -> TestStateBuilder {
        self.state
    }
    pub fn manual_delivery_points(self, amount: usize) -> ManualDeliveryPointsBuilder {
        self.state.manual_delivery_points(amount)
    }
    pub fn aqua_cultures(self, amount: usize) -> AquaCultureBuilder {
        self.state.aqua_cultures(amount)
    }
    pub fn vessels(self, amount: usize) -> VesselBuilder {
        self.state.vessels(amount)
    }
    pub async fn build(self) -> TestState {
        self.state.build().await
    }
    pub fn modify<F>(mut self, closure: F) -> MattilsynetBuilder
    where
        F: Fn(&mut MattilsynetConstructor),
    {
        self.state
            .mattilsynet
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(c));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> MattilsynetBuilder
    where
        F: Fn(usize, &mut MattilsynetConstructor),
    {
        self.state
            .mattilsynet
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, c));

        self
    }
}
