use crate::*;
use chrono::{DateTime, Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use std::collections::HashMap;

pub trait TestStorage:
    ScraperInboundPort + WebApiOutboundPort + AisConsumeLoop + Send + Sync + 'static
{
}

#[derive(Debug)]
pub struct TestState {
    pub ais_positions: Vec<AisPosition>,
    pub ais_vms_positions: Vec<crate::AisVmsPosition>,
    pub vessels: Vec<Vessel>,
    ais_positions_to_vessel: HashMap<VesselKey, Vec<AisPosition>>,
    ais_vms_positions_to_vessel: HashMap<VesselKey, Vec<crate::AisVmsPosition>>,
}

pub struct TestStateBuilder {
    storage: Box<dyn TestStorage>,
    vessels: Vec<VesselContructor>,
    ais_data_sender: tokio::sync::broadcast::Sender<DataMessage>,
    ais_data_confirmation: tokio::sync::mpsc::Receiver<()>,
    // Used for `vessel_id`, `call_sign` and `mmsi`
    vessel_id_counter: i64,
    position_timestamp_counter: HashMap<VesselKey, DateTime<Utc>>,
    position_gap: Duration,
    position_timestamp_start: DateTime<Utc>,
    ais_positions: HashMap<AisVesselKey, Vec<AisPositionConstructor>>,
    ais_vms_positions: HashMap<AisVmsVesselKey, Vec<AisVmsPositionConstructor>>,
    ais_static_messages: Vec<NewAisStatic>,
}

#[derive(Clone)]
pub enum AisVmsPosition {
    Ais(NewAisPosition),
    Vms(fiskeridir_rs::Vms),
}

pub struct VesselBuilder {
    state: TestStateBuilder,
    current_index: usize,
}

pub struct AisPositionBuilder {
    state: VesselBuilder,
    current_index: usize,
}

pub struct AisVmsPositionBuilder {
    state: VesselBuilder,
    current_index: usize,
}

struct AisPositionConstructor {
    index: usize,
    position: NewAisPosition,
}

#[derive(Clone)]
struct AisVmsPositionConstructor {
    index: usize,
    position: AisVmsPosition,
}

struct VesselContructor {
    key: VesselKey,
    mmsi: Mmsi,
    call_sign: CallSign,
    vessel: fiskeridir_rs::RegisterVessel,
}

#[derive(PartialEq, Eq, Hash)]
struct AisVmsVesselKey {
    mmsi: Mmsi,
    call_sign: CallSign,
    vessel_key: VesselKey,
}

#[derive(PartialEq, Eq, Hash)]
struct AisVesselKey {
    mmsi: Mmsi,
    vessel_key: VesselKey,
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
struct VesselKey {
    vessel_vec_index: usize,
}

impl AisVmsPosition {
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            AisVmsPosition::Ais(a) => a.msgtime,
            AisVmsPosition::Vms(v) => v.timestamp,
        }
    }
}

impl TestState {
    pub fn ais_positions_of_vessel(&self, index: usize) -> &[AisPosition] {
        self.ais_positions_to_vessel
            .get(&VesselKey {
                vessel_vec_index: index,
            })
            .unwrap()
    }
    pub fn ais_vms_positions_of_vessel(&self, index: usize) -> &[crate::AisVmsPosition] {
        self.ais_vms_positions_to_vessel
            .get(&VesselKey {
                vessel_vec_index: index,
            })
            .unwrap()
    }
}

impl TestStateBuilder {
    pub fn new(
        storage: Box<dyn TestStorage>,
        ais_consumer: Box<dyn AisConsumeLoop>,
    ) -> TestStateBuilder {
        let (sender, receiver) = tokio::sync::broadcast::channel::<DataMessage>(30);

        let (confirmation_sender, confirmation_receiver) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            ais_consumer
                .consume(receiver, Some(confirmation_sender))
                .await
        });
        TestStateBuilder {
            storage,
            ais_positions: HashMap::default(),
            vessels: vec![],
            vessel_id_counter: 1,
            position_timestamp_counter: HashMap::default(),
            ais_data_sender: sender,
            ais_data_confirmation: confirmation_receiver,
            position_gap: Duration::seconds(30),
            position_timestamp_start: Utc.with_ymd_and_hms(2010, 2, 5, 10, 0, 0).unwrap(),
            ais_static_messages: vec![],
            ais_vms_positions: HashMap::default(),
        }
    }

    pub fn position_increments(mut self, duration: Duration) -> TestStateBuilder {
        self.position_gap = duration;
        self
    }

    pub fn position_start(mut self, time: DateTime<Utc>) -> TestStateBuilder {
        self.position_timestamp_start = time;
        self
    }

    pub fn vessels(mut self, amount: usize) -> VesselBuilder {
        let num_vessels = self.vessels.len();
        for i in 0..amount {
            let vessel_id = self.vessel_id_counter;

            let mut vessel = fiskeridir_rs::RegisterVessel::test_default(vessel_id);
            let call_sign = CallSign::try_from(format!("CS{}", self.vessel_id_counter)).unwrap();
            let mmsi = Mmsi(self.vessel_id_counter as i32);
            let ais_static = NewAisStatic::test_default(mmsi, call_sign.as_ref());
            vessel.radio_call_sign = Some(call_sign.clone());

            self.ais_static_messages.push(ais_static);

            self.vessels.push(VesselContructor {
                key: VesselKey {
                    vessel_vec_index: num_vessels + i,
                },
                mmsi,
                vessel,
                call_sign,
            });

            self.position_timestamp_counter.insert(
                VesselKey {
                    vessel_vec_index: i + num_vessels,
                },
                self.position_timestamp_start,
            );

            self.vessel_id_counter += 1;
        }

        VesselBuilder {
            current_index: self.vessels.len() - amount,
            state: self,
        }
    }

    pub async fn build(mut self) -> TestState {
        self.ais_data_sender
            .send(DataMessage {
                positions: vec![],
                static_messages: self.ais_static_messages,
            })
            .unwrap();

        self.ais_data_confirmation.recv().await.unwrap();

        let num_vessels = self.vessels.len();
        self.storage
            .add_register_vessels(self.vessels.into_iter().map(|v| v.vessel).collect())
            .await
            .unwrap();
        let mut vessels: Vec<Vessel> = self.storage.vessels().try_collect().await.unwrap();
        vessels.sort_by_key(|v| v.fiskeridir.id);

        let mut ais_positions_by_vessel = HashMap::default();
        let mut ais_positions = Vec::new();

        let mut ais_vms_positions_by_vessel = HashMap::default();
        let mut ais_vms_positions = Vec::new();

        for (key, positions) in self.ais_positions {
            let start = &positions[0].position.msgtime;
            let end = &positions[positions.len() - 1].position.msgtime;
            let range = DateRange::new(*start, *end).unwrap();

            let num_positions = positions.len();

            self.ais_data_sender
                .send(DataMessage {
                    positions: positions.into_iter().map(|v| v.position).collect(),
                    static_messages: vec![],
                })
                .unwrap();

            self.ais_data_confirmation.recv().await.unwrap();

            let mut stored_positions: Vec<AisPosition> = self
                .storage
                .ais_positions(key.mmsi, &range)
                .try_collect()
                .await
                .unwrap();

            assert_eq!(stored_positions.len(), num_positions);
            ais_positions_by_vessel.insert(key.vessel_key, stored_positions.clone());
            ais_positions.append(&mut stored_positions);
        }

        for (key, positions) in self.ais_vms_positions {
            let start = &positions[0].position.timestamp();
            let end = &positions[positions.len() - 1].position.timestamp();

            let range = DateRange::new(*start, *end).unwrap();

            let num_positions = positions.len();

            let ais: Vec<NewAisPosition> = positions
                .iter()
                .cloned()
                .filter_map(|v| match v.position {
                    AisVmsPosition::Ais(a) => Some(a),
                    AisVmsPosition::Vms(_) => None,
                })
                .collect();

            let vms: Vec<fiskeridir_rs::Vms> = positions
                .into_iter()
                .filter_map(|v| match v.position {
                    AisVmsPosition::Vms(a) => Some(a),
                    AisVmsPosition::Ais(_) => None,
                })
                .collect();

            self.ais_data_sender
                .send(DataMessage {
                    positions: ais,
                    static_messages: vec![],
                })
                .unwrap();

            self.ais_data_confirmation.recv().await.unwrap();

            self.storage.add_vms(vms).await.unwrap();

            let stored_positions = self
                .storage
                .ais_vms_positions(Some(key.mmsi), Some(&key.call_sign), &range)
                .try_collect::<Vec<crate::AisVmsPosition>>()
                .await
                .unwrap();

            let mut mmsi_mapped_positions: Vec<(Mmsi, crate::AisVmsPosition)> = stored_positions
                .iter()
                .cloned()
                .map(|v| (key.mmsi, v))
                .collect();

            assert_eq!(stored_positions.len(), num_positions);
            ais_vms_positions_by_vessel.insert(key.vessel_key, stored_positions);
            ais_vms_positions.append(&mut mmsi_mapped_positions);
        }

        // We want all positions to be ordered by how they were created, we exploit the fact that
        // mmsis are an increasing counter and that msgtime is increased for each created position.
        ais_positions.sort_by_key(|v| (v.mmsi, v.msgtime));
        ais_vms_positions.sort_by_key(|v| (v.0, v.1.timestamp));
        assert_eq!(vessels.len(), num_vessels);

        TestState {
            ais_positions,
            vessels,
            ais_positions_to_vessel: ais_positions_by_vessel,
            ais_vms_positions: ais_vms_positions.into_iter().map(|v| v.1).collect(),
            ais_vms_positions_to_vessel: ais_vms_positions_by_vessel,
        }
    }
}

impl VesselBuilder {
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

            for i in 0..num_positions {
                timestamps.push(*timestamp);
                let position = if (i + 1) % 2 == 0 {
                    AisVmsPosition::Vms(fiskeridir_rs::Vms::test_default(
                        rand::random(),
                        vessel.call_sign.clone(),
                        *timestamp,
                    ))
                } else {
                    AisVmsPosition::Ais(NewAisPosition::test_default(vessel.mmsi, *timestamp))
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
                    mmsi: vessel.mmsi,
                    vessel_key: vessel.key,
                    call_sign: vessel.call_sign.clone(),
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
                let position = NewAisPosition::test_default(vessel.mmsi, *timestamp);
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
                    mmsi: vessel.mmsi,
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

    pub async fn build(self) -> TestState {
        self.state.build().await
    }
}

impl AisVmsPositionBuilder {
    pub fn modify<F>(mut self, closure: F) -> AisVmsPositionBuilder
    where
        F: Fn(&mut AisVmsPosition),
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
        F: Fn(usize, &mut AisVmsPosition),
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
                    .filter(|v| v.index < self.current_index)
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

fn positions_per_vessel(amount: usize, num_vessels: usize) -> (usize, usize) {
    let per_vessel = amount / num_vessels;
    let remainder = if amount > num_vessels && amount % num_vessels != 0 {
        (amount % num_vessels) + per_vessel
    } else {
        per_vessel
    };

    (per_vessel, remainder)
}
