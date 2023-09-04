use super::{vessel::VesselBuilder, VesselKey};
use crate::*;

pub struct AisPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct AisPositionConstructor {
    pub index: usize,
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
            .for_each(|(_, positions)| {
                for p in positions
                    .iter_mut()
                    .filter(|v| v.index >= self.current_index)
                {
                    closure(&mut p.position)
                }
            });

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
            .for_each(|(_, positions)| {
                for p in positions
                    .iter_mut()
                    .filter(|v| v.index >= self.current_index)
                {
                    closure(p.index, &mut p.position)
                }
            });

        self
    }

    pub async fn build(self) -> TestState {
        self.state.state.build().await
    }
}
