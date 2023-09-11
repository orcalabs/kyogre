use super::*;

pub struct VmsPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone)]
pub struct VmsPositionConstructor {
    pub position: fiskeridir_rs::Vms,
}

#[derive(PartialEq, Eq, Hash)]
pub struct VmsVesselKey {
    pub call_sign: CallSign,
    pub vessel_key: VesselKey,
}

impl VmsPositionBuilder {
    pub fn modify<F>(mut self, closure: F) -> VmsPositionBuilder
    where
        F: Fn(&mut fiskeridir_rs::Vms),
    {
        self.state
            .state
            .vms_positions
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| *i >= self.current_index)
            .for_each(|(_, c)| closure(&mut c.position));

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> VmsPositionBuilder
    where
        F: Fn(usize, &mut fiskeridir_rs::Vms),
    {
        self.state
            .state
            .vms_positions
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
