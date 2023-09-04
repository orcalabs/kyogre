use super::*;

pub struct VmsPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone)]
pub struct VmsPositionConstructor {
    pub index: usize,
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

    pub fn modify_idx<F>(mut self, closure: F) -> VmsPositionBuilder
    where
        F: Fn(usize, &mut fiskeridir_rs::Vms),
    {
        self.state
            .state
            .vms_positions
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
