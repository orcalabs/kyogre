use super::{vessel::VesselBuilder, VesselKey};
use crate::*;

pub struct AisPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct AisPositionConstructor {
    pub position: NewAisPosition,
}

#[derive(PartialEq, Eq, Hash)]
pub struct AisVesselKey {
    pub mmsi: Mmsi,
    pub vessel_key: VesselKey,
}

impl AisPositionBuilder {
    pub fn modify<F>(mut self, closure: F) -> AisPositionBuilder
    where
        F: Fn(&mut NewAisPosition),
    {
        self.state
            .state
            .ais_positions
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(&mut c.position));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> AisPositionBuilder
    where
        F: Fn(usize, &mut NewAisPosition),
    {
        self.state
            .state
            .ais_positions
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(idx, c)| closure(idx, &mut c.position));

        self
    }

    pub async fn build(self) -> TestState {
        self.state.state.build().await
    }
}
