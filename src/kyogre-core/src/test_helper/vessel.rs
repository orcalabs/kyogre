use super::ais_vms::AisOrVmsPosition;
use super::*;

pub struct VesselBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct VesselKey {
    pub vessel_vec_index: usize,
}

pub struct VesselContructor {
    pub key: VesselKey,
    pub fiskeridir: fiskeridir_rs::RegisterVessel,
    pub ais: NewAisStatic,
}

impl VesselBuilder {
    pub fn vms_positions(mut self, amount: usize) -> VmsPositionBuilder {
        assert!(amount != 0);

        let num_vessels = self.state.vessels[self.current_index..].len();

        let (per_vessel, remainder) = positions_per_vessel(amount, num_vessels);

        let mut current_position_index = 0;
        for (i, vessel) in self.state.vessels[self.current_index..].iter().enumerate() {
            let num_positions = if i == num_vessels - 1 {
                remainder
            } else {
                per_vessel
            };

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let timestamp = self
                .state
                .position_timestamp_counter
                .get_mut(&vessel.key)
                .unwrap();

            let call_sign = vessel.fiskeridir.radio_call_sign.clone().unwrap();

            for _ in 0..num_positions {
                timestamps.push(*timestamp);
                let position =
                    fiskeridir_rs::Vms::test_default(rand::random(), call_sign.clone(), *timestamp);
                *timestamp += self.state.position_gap;
                positions.push(VmsPositionConstructor {
                    index: current_position_index,
                    position,
                });
                current_position_index += 1;
            }

            self.state
                .vms_positions
                .entry(VmsVesselKey {
                    vessel_key: vessel.key,
                    call_sign,
                })
                .and_modify(|v| v.append(&mut positions))
                .or_insert(positions);
        }

        VmsPositionBuilder {
            current_index: self.current_index + amount,
            state: self,
        }
    }
    pub fn ais_vms_positions(mut self, amount: usize) -> AisVmsPositionBuilder {
        assert!(amount != 0);

        let num_vessels = self.state.vessels[self.current_index..].len();

        let (per_vessel, remainder) = positions_per_vessel(amount, num_vessels);

        let mut current_position_index = 0;
        for (i, vessel) in self.state.vessels[self.current_index..].iter().enumerate() {
            let num_positions = if i == num_vessels - 1 {
                remainder
            } else {
                per_vessel
            };

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let timestamp = self
                .state
                .position_timestamp_counter
                .get_mut(&vessel.key)
                .unwrap();

            let call_sign = vessel.fiskeridir.radio_call_sign.clone().unwrap();

            for i in 0..num_positions {
                timestamps.push(*timestamp);
                let position = if (i + 1) % 2 == 0 {
                    AisOrVmsPosition::Vms(fiskeridir_rs::Vms::test_default(
                        rand::random(),
                        call_sign.clone(),
                        *timestamp,
                    ))
                } else {
                    AisOrVmsPosition::Ais(NewAisPosition::test_default(vessel.ais.mmsi, *timestamp))
                };
                *timestamp += self.state.position_gap;
                positions.push(AisVmsPositionConstructor {
                    index: current_position_index,
                    position,
                });
                current_position_index += 1;
            }

            self.state
                .ais_vms_positions
                .entry(AisVmsVesselKey {
                    mmsi: vessel.ais.mmsi,
                    vessel_key: vessel.key,
                    call_sign,
                })
                .and_modify(|v| v.append(&mut positions))
                .or_insert(positions);
        }

        AisVmsPositionBuilder {
            current_index: self.current_index + amount,
            state: self,
        }
    }
    pub fn ais_positions(mut self, amount: usize) -> AisPositionBuilder {
        assert!(amount != 0);

        let num_vessels = self.state.vessels[self.current_index..].len();

        let (per_vessel, remainder) = positions_per_vessel(amount, num_vessels);

        let mut current_position_index = 0;

        for (i, vessel) in self.state.vessels[self.current_index..].iter().enumerate() {
            let num_positions = if i == num_vessels - 1 {
                remainder
            } else {
                per_vessel
            };

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let timestamp = self
                .state
                .position_timestamp_counter
                .get_mut(&vessel.key)
                .unwrap();

            for _ in 0..num_positions {
                timestamps.push(*timestamp);
                let position = NewAisPosition::test_default(vessel.ais.mmsi, *timestamp);
                *timestamp += self.state.position_gap;
                positions.push(AisPositionConstructor {
                    index: current_position_index,
                    position,
                });
                current_position_index += 1;
            }

            self.state
                .ais_positions
                .entry(AisVesselKey {
                    mmsi: vessel.ais.mmsi,
                    vessel_key: vessel.key,
                })
                .and_modify(|v| v.append(&mut positions))
                .or_insert(positions);
        }

        AisPositionBuilder {
            current_index: self.current_index + amount,
            state: self,
        }
    }

    pub fn modify<F>(mut self, closure: F) -> VesselBuilder
    where
        F: Fn(&mut VesselContructor),
    {
        self.state
            .vessels
            .iter_mut()
            .enumerate()
            .for_each(|(i, vessel)| {
                if i <= self.current_index {
                    closure(vessel)
                }
            });
        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> VesselBuilder
    where
        F: Fn(usize, &mut VesselContructor),
    {
        self.state
            .vessels
            .iter_mut()
            .enumerate()
            .for_each(|(i, vessel)| {
                if i >= self.current_index {
                    closure(i, vessel)
                }
            });
        self
    }

    pub async fn build(self) -> TestState {
        self.state.build().await
    }
}
