use crate::{
    AisConsumeLoop, AisPermission, AisPosition, DataMessage, DateRange, FisheryEngine,
    FiskeridirVesselId, Haul, HaulsQuery, Mmsi, NewAisPosition, NewAisStatic, Pagination,
    ScraperInboundPort, TripAssemblerOutboundPort, TripDetailed, Trips, TripsQuery, Vessel,
    VmsPosition, WebApiOutboundPort,
};
use ais::*;
use ais_vms::*;
use chrono::{DateTime, Duration, TimeZone, Utc};
use fiskeridir_rs::LandingMonth;
use fiskeridir_rs::{CallSign, LandingId};
use futures::TryStreamExt;
use machine::StateMachine;
use std::collections::HashMap;

pub trait TestStorage:
    ScraperInboundPort
    + WebApiOutboundPort
    + AisConsumeLoop
    + TripAssemblerOutboundPort
    + Send
    + Sync
    + 'static
{
}

mod ais;
mod ais_vms;
mod fishing_facility;
mod haul;
mod item_distribution;
mod landing;
mod tra;
mod trip;
mod vessel;
mod vms;

pub use ais::*;
pub use ais_vms::*;
pub use fishing_facility::*;
pub use haul::*;
pub use landing::*;
pub use tra::*;
pub use trip::*;
pub use vessel::*;
pub use vms::*;

#[derive(Debug)]
pub struct TestState {
    pub vms_positions: Vec<VmsPosition>,
    pub ais_positions: Vec<AisPosition>,
    pub ais_vms_positions: Vec<crate::AisVmsPosition>,
    pub vessels: Vec<Vessel>,
    pub trips: Vec<TripDetailed>,
    pub landing_ids: Vec<LandingId>,
    pub hauls: Vec<Haul>,
    ais_positions_to_vessel: HashMap<VesselKey, Vec<AisPosition>>,
    ais_vms_positions_to_vessel: HashMap<VesselKey, Vec<crate::AisVmsPosition>>,
    vms_positions_to_vessel: HashMap<VesselKey, Vec<crate::VmsPosition>>,
    trips_to_vessel: HashMap<VesselKey, Vec<TripDetailed>>,
}

#[allow(dead_code)]
pub struct TestStateBuilder {
    storage: Box<dyn TestStorage>,
    vessels: Vec<VesselContructor>,
    ais_data_sender: tokio::sync::broadcast::Sender<DataMessage>,
    ais_data_confirmation: tokio::sync::mpsc::Receiver<()>,
    // Used for `vessel_id`, `call_sign` and `mmsi`
    vessel_id_counter: i64,
    global_data_timestamp_counter: DateTime<Utc>,
    data_timestamp_gap: Duration,
    ais_vms_positions: HashMap<AisVmsVesselKey, Vec<AisVmsPositionConstructor>>,
    ais_positions: HashMap<AisVesselKey, Vec<AisPositionConstructor>>,
    vms_positions: HashMap<VmsVesselKey, Vec<VmsPositionConstructor>>,
    trips: Vec<TripConstructor>,
    hauls: Vec<HaulConstructor>,
    landings: Vec<fiskeridir_rs::Landing>,
    tra: Vec<TraConstructor>,
    fishing_facilities: Vec<FishingFacilityConctructor>,
    default_trip_duration: Duration,
    default_haul_duration: Duration,
    default_fishing_facility_duration: Duration,
    trip_data_timestamp_gap: Duration,
    ers_message_id_counter: u64,
    ers_message_number_per_vessel: HashMap<VesselKey, u32>,
    landing_id_counter: u64,
    engine: FisheryEngine,
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
    pub fn trips_of_vessel(&self, index: usize) -> &[TripDetailed] {
        self.trips_to_vessel
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
        engine: FisheryEngine,
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
            ais_data_sender: sender,
            ais_data_confirmation: confirmation_receiver,
            data_timestamp_gap: Duration::seconds(30),
            ais_vms_positions: HashMap::default(),
            vms_positions: HashMap::default(),
            trips: vec![],
            default_trip_duration: Duration::weeks(1),
            ers_message_id_counter: 1,
            ers_message_number_per_vessel: HashMap::default(),
            engine,
            landings: vec![],
            landing_id_counter: 1,
            trip_data_timestamp_gap: Duration::hours(1),
            hauls: vec![],
            default_haul_duration: Duration::hours(1),
            tra: vec![],
            global_data_timestamp_counter: Utc.with_ymd_and_hms(2010, 2, 5, 10, 0, 0).unwrap(),
            fishing_facilities: vec![],
            default_fishing_facility_duration: Duration::hours(1),
        }
    }

    pub fn data_increment(mut self, duration: Duration) -> TestStateBuilder {
        self.data_timestamp_gap = duration;
        self
    }

    pub fn data_start(mut self, time: DateTime<Utc>) -> TestStateBuilder {
        self.global_data_timestamp_counter = time;
        self
    }

    pub fn landings(mut self, amount: usize) -> LandingBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let mut landing =
                fiskeridir_rs::Landing::test_default(self.landing_id_counter as i64, None);

            let ts = self.global_data_timestamp_counter;
            landing.landing_timestamp = ts;
            landing.landing_time = ts.time();
            landing.landing_month = LandingMonth::from(ts);

            self.landings.push(landing);

            self.landing_id_counter += 1;

            self.global_data_timestamp_counter += self.data_timestamp_gap;
        }

        LandingBuilder {
            current_index: self.landings.len() - amount,
            state: self,
        }
    }
    pub fn tra(mut self, amount: usize) -> TraBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let timestamp = self.global_data_timestamp_counter;
            let mut tra =
                fiskeridir_rs::ErsTra::test_default(self.ers_message_id_counter, None, timestamp);

            tra.message_info.set_message_timestamp(timestamp);

            self.ers_message_id_counter += 1;

            self.tra.push(TraConstructor { tra });
            self.global_data_timestamp_counter += self.data_timestamp_gap;
        }

        TraBuilder {
            current_index: self.tra.len() - amount,
            state: self,
        }
    }
    pub fn hauls(mut self, amount: usize) -> HaulBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let timestamp = self.global_data_timestamp_counter;
            let mut dca = fiskeridir_rs::ErsDca::test_default(self.ers_message_id_counter, None);

            self.ers_message_id_counter += 1;
            let start = timestamp;
            let end = timestamp + self.default_haul_duration;
            dca.message_info.set_message_timestamp(start);
            dca.set_start_timestamp(start);

            dca.set_stop_timestamp(end);

            self.hauls.push(HaulConstructor { dca });

            self.global_data_timestamp_counter = end + self.data_timestamp_gap;
        }

        HaulBuilder {
            current_index: self.hauls.len() - amount,
            state: self,
        }
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

            let key = VesselKey {
                vessel_vec_index: num_vessels + i,
            };
            self.vessels.push(VesselContructor {
                key,
                fiskeridir: vessel,
                ais: ais_static,
            });

            self.ers_message_number_per_vessel.insert(key, 1);

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

        let mut ais_positions_to_vessel = HashMap::default();
        let mut ais_positions = Vec::new();

        let mut ais_vms_positions_to_vessel = HashMap::default();
        let mut ais_vms_positions = Vec::new();

        let mut vms_positions_to_vessel = HashMap::default();
        let mut vms_positions = Vec::new();

        let mut trips_to_vessel: HashMap<VesselKey, Vec<TripDetailed>> =
            HashMap::with_capacity(self.trips.len());
        let mut trips = Vec::new();

        self.storage
            .add_ers_dca(Box::new(self.hauls.into_iter().map(|v| Ok(v.dca))))
            .await
            .unwrap();

        self.storage
            .add_ers_tra(self.tra.into_iter().map(|v| v.tra).collect())
            .await
            .unwrap();

        self.storage
            .add_fishing_facilities(
                self.fishing_facilities
                    .into_iter()
                    .map(|v| v.facility)
                    .collect(),
            )
            .await
            .unwrap();

        for t in self.trips {
            match t.trip_specification {
                TripSpecification::Ers { dep, por } => {
                    self.storage.add_ers_dep(vec![dep]).await.unwrap();
                    self.storage.add_ers_por(vec![por]).await.unwrap();
                }
                TripSpecification::Landing {
                    start_landing,
                    end_landing,
                } => {
                    self.landings.append(&mut vec![start_landing, end_landing]);
                }
            }
        }

        let mut landing_ids: Vec<(i64, DateTime<Utc>, LandingId)> = self
            .landings
            .iter()
            .map(|l| (l.vessel.id.unwrap_or(0), l.landing_timestamp, l.id.clone()))
            .collect();

        self.storage
            .add_landings(Box::new(self.landings.into_iter().map(Ok)), 2023)
            .await
            .unwrap();

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
                .ais_positions(key.mmsi, &range, AisPermission::All)
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
                .ais_vms_positions(
                    Some(key.mmsi),
                    Some(&key.call_sign),
                    &range,
                    AisPermission::All,
                )
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

        let mut engine = self.engine.run_single().await;
        loop {
            if engine.current_state_name() == "Pending" {
                break;
            }
            engine = engine.run_single().await;
        }

        let mut vessels: Vec<Vessel> = self.storage.vessels().try_collect().await.unwrap();
        vessels.sort_by_key(|v| v.fiskeridir.id);

        let vessel_ids_keys: HashMap<FiskeridirVesselId, VesselKey> = vessels
            .iter()
            .enumerate()
            .map(|(i, v)| {
                (
                    v.fiskeridir.id,
                    VesselKey {
                        vessel_vec_index: i,
                    },
                )
            })
            .collect();

        let vessel_trips = self
            .storage
            .detailed_trips(
                TripsQuery {
                    pagination: Pagination::<Trips>::new(Some(100), None),
                    ..Default::default()
                },
                true,
            )
            .unwrap()
            .try_collect::<Vec<TripDetailed>>()
            .await
            .unwrap();

        assert!(vessel_trips.len() < 100);

        for t in vessel_trips {
            let key = vessel_ids_keys.get(&t.fiskeridir_vessel_id).unwrap();
            trips.push(t.clone());
            trips_to_vessel
                .entry(*key)
                .and_modify(|v| v.push(t.clone()))
                .or_insert(vec![t]);
        }

        let mut hauls = self
            .storage
            .hauls(HaulsQuery::default())
            .unwrap()
            .try_collect::<Vec<Haul>>()
            .await
            .unwrap();

        // We want all positions to be ordered by how they were created, we exploit the fact that
        // mmsis are an increasing counter and that msgtime is increased for each created position.
        ais_positions.sort_by_key(|v| (v.mmsi, v.msgtime));
        ais_vms_positions.sort_by_key(|v| (v.0, v.1.timestamp));
        vms_positions.sort_by_key(|v| (v.call_sign.clone(), v.timestamp));
        trips.sort_by_key(|v| (v.fiskeridir_vessel_id, v.period.start()));
        landing_ids.sort_by_key(|v| (v.0, v.1));
        hauls.sort_by_key(|v| (v.start_timestamp, v.haul_id));
        assert_eq!(vessels.len(), num_vessels);

        TestState {
            ais_positions,
            vessels,
            ais_positions_to_vessel,
            ais_vms_positions: ais_vms_positions.into_iter().map(|v| v.1).collect(),
            ais_vms_positions_to_vessel,
            vms_positions,
            vms_positions_to_vessel,
            trips,
            trips_to_vessel,
            landing_ids: landing_ids.into_iter().map(|v| v.2).collect(),
            hauls,
        }
    }
}
