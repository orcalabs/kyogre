use super::*;

pub struct AisVmsPositionBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone)]
pub enum AisOrVmsPosition {
    Ais(NewAisPosition),
    Vms(fiskeridir_rs::Vms),
}

#[derive(Clone)]
pub struct AisVmsPositionConstructor {
    pub index: usize,
    pub position: AisOrVmsPosition,
}

#[derive(PartialEq, Eq, Hash)]
pub struct AisVmsVesselKey {
    pub mmsi: Mmsi,
    pub call_sign: CallSign,
    pub vessel_key: VesselKey,
}

impl AisOrVmsPosition {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            AisOrVmsPosition::Ais(a) => a.msgtime,
            AisOrVmsPosition::Vms(v) => v.timestamp,
        }
    }
    pub fn set_timestamp(&mut self, timestamp: DateTime<Utc>) {
        match self {
            AisOrVmsPosition::Ais(a) => a.msgtime = timestamp,
            AisOrVmsPosition::Vms(v) => v.timestamp = timestamp,
        }
    }
}

impl AisVmsPositionBuilder {
    pub fn modify<F>(mut self, closure: F) -> AisVmsPositionBuilder
    where
        F: Fn(&mut AisOrVmsPosition),
    {
        self.state
            .state
            .ais_vms_positions
            .iter_mut()
            .for_each(|(_, positions)| {
                for p in positions
                    .iter_mut()
                    .filter(|v| v.index < self.current_index)
                {
                    closure(&mut p.position)
                }
            });

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> AisVmsPositionBuilder
    where
        F: Fn(usize, &mut AisOrVmsPosition),
    {
        self.state
            .state
            .ais_vms_positions
            .iter_mut()
            .for_each(|(_, positions)| {
                for p in positions
                    .iter_mut()
                    .filter(|v| v.index < self.current_index)
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
