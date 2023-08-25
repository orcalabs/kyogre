use crate::{
    AisConsumeLoop, AisPosition, DataMessage, DateRange, Mmsi, NewAisPosition, NewAisStatic,
    ScraperInboundPort, Vessel, VmsPosition, WebApiOutboundPort,
};
use ais::*;
pub use ais_vms::AisOrVmsPosition;
use ais_vms::*;
use chrono::{DateTime, Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use std::collections::HashMap;
use vessel::*;
use vms::*;

pub trait TestStorage:
    ScraperInboundPort + WebApiOutboundPort + AisConsumeLoop + Send + Sync + 'static
{
}

mod ais;
mod ais_vms;
mod vessel;
mod vms;

#[derive(Debug)]
pub struct TestState {
    pub vms_positions: Vec<VmsPosition>,
    pub ais_positions: Vec<AisPosition>,
    pub ais_vms_positions: Vec<crate::AisVmsPosition>,
    pub vessels: Vec<Vessel>,
    ais_positions_to_vessel: HashMap<VesselKey, Vec<AisPosition>>,
    ais_vms_positions_to_vessel: HashMap<VesselKey, Vec<crate::AisVmsPosition>>,
    vms_positions_to_vessel: HashMap<VesselKey, Vec<crate::VmsPosition>>,
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
    ais_vms_positions: HashMap<AisVmsVesselKey, Vec<AisVmsPositionConstructor>>,
    ais_positions: HashMap<AisVesselKey, Vec<AisPositionConstructor>>,
    vms_positions: HashMap<VmsVesselKey, Vec<VmsPositionConstructor>>,
}

impl TestState {
    pub fn vms_positions_of_vessel(&self, index: usize) -> &[VmsPosition] {
        self.vms_positions_to_vessel
            .get(&VesselKey {
                vessel_vec_index: index,
            })
            .unwrap()
    }
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
            ais_vms_positions: HashMap::default(),
            vms_positions: HashMap::default(),
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

            self.vessels.push(VesselContructor {
                key: VesselKey {
                    vessel_vec_index: num_vessels + i,
                },
                fiskeridir: vessel,
                ais: ais_static,
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
                static_messages: self.vessels.iter().map(|v| v.ais.clone()).collect(),
            })
            .unwrap();

        self.ais_data_confirmation.recv().await.unwrap();

        let num_vessels = self.vessels.len();
        self.storage
            .add_register_vessels(self.vessels.into_iter().map(|v| v.fiskeridir).collect())
            .await
            .unwrap();
        let mut vessels: Vec<Vessel> = self.storage.vessels().try_collect().await.unwrap();
        vessels.sort_by_key(|v| v.fiskeridir.id);

        let mut ais_positions_to_vessel = HashMap::default();
        let mut ais_positions = Vec::new();

        let mut ais_vms_positions_to_vessel = HashMap::default();
        let mut ais_vms_positions = Vec::new();

        let mut vms_positions_to_vessel = HashMap::default();
        let mut vms_positions = Vec::new();

        for (key, positions) in self.ais_positions {
            let start = &positions[0].position.msgtime;
            let end = &positions[positions.len() - 1].position.msgtime;
            let range = DateRange::new(*start, *end).unwrap();

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

            ais_positions_to_vessel.insert(key.vessel_key, stored_positions.clone());
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
                    AisOrVmsPosition::Ais(a) => Some(a),
                    AisOrVmsPosition::Vms(_) => None,
                })
                .collect();

            let vms: Vec<fiskeridir_rs::Vms> = positions
                .into_iter()
                .filter_map(|v| match v.position {
                    AisOrVmsPosition::Vms(a) => Some(a),
                    AisOrVmsPosition::Ais(_) => None,
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
            ais_vms_positions_to_vessel.insert(key.vessel_key, stored_positions);
            ais_vms_positions.append(&mut mmsi_mapped_positions);
        }

        for (key, positions) in self.vms_positions {
            let start = &positions[0].position.timestamp;
            let end = &positions[positions.len() - 1].position.timestamp;

            let range = DateRange::new(*start, *end).unwrap();
            let num_positions = positions.len();

            self.storage
                .add_vms(positions.iter().cloned().map(|v| v.position).collect())
                .await
                .unwrap();

            let mut stored_positions = self
                .storage
                .vms_positions(&key.call_sign, &range)
                .try_collect::<Vec<VmsPosition>>()
                .await
                .unwrap();

            assert_eq!(stored_positions.len(), num_positions);
            vms_positions_to_vessel.insert(key.vessel_key, stored_positions.clone());
            vms_positions.append(&mut stored_positions);
        }

        // We want all positions to be ordered by how they were created, we exploit the fact that
        // mmsis are an increasing counter and that msgtime is increased for each created position.
        ais_positions.sort_by_key(|v| (v.mmsi, v.msgtime));
        ais_vms_positions.sort_by_key(|v| (v.0, v.1.timestamp));
        vms_positions.sort_by_key(|v| (v.call_sign.clone(), v.timestamp));
        assert_eq!(vessels.len(), num_vessels);

        TestState {
            ais_positions,
            vessels,
            ais_positions_to_vessel,
            ais_vms_positions: ais_vms_positions.into_iter().map(|v| v.1).collect(),
            ais_vms_positions_to_vessel,
            vms_positions,
            vms_positions_to_vessel,
        }
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
