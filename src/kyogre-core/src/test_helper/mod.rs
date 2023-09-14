use crate::{
    AisConsumeLoop, AisPosition, Arrival, DataMessage, DeliveryPoint, DeliveryPointType, Departure,
    FisheryEngine, FishingFacilities, FishingFacilitiesQuery, FishingFacility, Haul, HaulsQuery,
    Landing, LandingsQuery, LandingsSorting, ManualDeliveryPoint, MattilsynetDeliveryPoint, Mmsi,
    NewAisPosition, NewAisStatic, Ordering, Pagination, PrecisionId, ScraperInboundPort,
    TestHelperInbound, TestHelperOutbound, TripAssemblerOutboundPort, TripDetailed, Trips,
    TripsQuery, Vessel, VmsPosition, WebApiOutboundPort,
};
use ais::*;
use ais_vms::*;
use chrono::{DateTime, Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use fiskeridir_rs::{DeliveryPointId, LandingMonth};
use futures::TryStreamExt;
use machine::StateMachine;
use std::collections::{HashMap, HashSet};

pub trait TestStorage:
    ScraperInboundPort
    + WebApiOutboundPort
    + AisConsumeLoop
    + TripAssemblerOutboundPort
    + TestHelperOutbound
    + TestHelperInbound
    + Send
    + Sync
    + 'static
{
}

mod ais;
mod ais_vms;
mod delivery_points;
mod dep;
mod fishing_facility;
mod haul;
mod item_distribution;
mod landing;
mod por;
mod tra;
mod trip;
mod vessel;
mod vms;

pub use ais::*;
pub use ais_vms::*;
pub use delivery_points::*;
pub use dep::*;
pub use fishing_facility::*;
pub use haul::*;
pub use landing::*;
pub use por::*;
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
    pub landings: Vec<Landing>,
    pub hauls: Vec<Haul>,
    pub dep: Vec<Departure>,
    pub por: Vec<Arrival>,
    // Includes the static delivery points from our migrations
    pub all_delivery_points: Vec<DeliveryPoint>,
    // Only includes the delivery points added by the builder
    pub delivery_points: Vec<DeliveryPoint>,
    pub fishing_facilities: Vec<FishingFacility>,
}

pub struct TestStateBuilder {
    storage: Box<dyn TestStorage>,
    vessels: Vec<VesselContructor>,
    ais_data_sender: tokio::sync::broadcast::Sender<DataMessage>,
    ais_data_confirmation: tokio::sync::mpsc::Receiver<()>,
    // Used for `vessel_id`, `call_sign` and `mmsi`
    vessel_id_counter: i64,
    global_data_timestamp_counter: DateTime<Utc>,
    data_timestamp_gap: Duration,
    ais_vms_positions: Vec<AisVmsPositionConstructor>,
    ais_positions: Vec<AisPositionConstructor>,
    vms_positions: Vec<VmsPositionConstructor>,
    trips: Vec<TripConstructor>,
    hauls: Vec<HaulConstructor>,
    landings: Vec<fiskeridir_rs::Landing>,
    tra: Vec<TraConstructor>,
    dep: Vec<DepConstructor>,
    por: Vec<PorConstructor>,
    aqua_cultures: Vec<AquaCultureConstructor>,
    mattilsynet: Vec<MattilsynetConstructor>,
    manual_delivery_points: Vec<ManualDeliveryPointConstructor>,
    fishing_facilities: Vec<FishingFacilityConctructor>,
    default_trip_duration: Duration,
    default_haul_duration: Duration,
    default_fishing_facility_duration: Duration,
    trip_data_timestamp_gap: Duration,
    ers_message_id_counter: u64,
    ers_message_number_per_vessel: HashMap<VesselKey, u32>,
    delivery_point_id_counter: u64,
    landing_id_counter: u64,
    engine: FisheryEngine,
}

enum TripPrecisonStartPoint {
    Port {
        start_port: String,
        end_port: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        mmsi: Mmsi,
    },
    DockPoint {
        start_port: String,
        end_port: String,
        end: DateTime<Utc>,
        start: DateTime<Utc>,
        mmsi: Mmsi,
    },
    DeliveryPoint {
        id: DeliveryPointId,
        end: DateTime<Utc>,
        mmsi: Mmsi,
    },
    FirstPoint {
        start: DateTime<Utc>,
        mmsi: Mmsi,
    },
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
            ais_positions: vec![],
            vessels: vec![],
            vessel_id_counter: 1,
            ais_data_sender: sender,
            ais_data_confirmation: confirmation_receiver,
            data_timestamp_gap: Duration::seconds(30),
            ais_vms_positions: vec![],
            vms_positions: vec![],
            trips: vec![],
            default_trip_duration: Duration::weeks(1),
            ers_message_id_counter: 1,
            delivery_point_id_counter: 1,
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
            dep: vec![],
            por: vec![],
            aqua_cultures: vec![],
            mattilsynet: vec![],
            manual_delivery_points: vec![],
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

    pub fn trip_duration(mut self, duration: Duration) -> TestStateBuilder {
        self.default_trip_duration = duration;
        self
    }

    pub fn mattilsynet(mut self, amount: usize) -> MattilsynetBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let mut val = MattilsynetDeliveryPoint::test_default();
            val.id =
                DeliveryPointId::try_from(format!("DP{}", self.delivery_point_id_counter)).unwrap();
            self.delivery_point_id_counter += 1;
            self.mattilsynet.push(MattilsynetConstructor { val });
        }

        MattilsynetBuilder {
            current_index: self.mattilsynet.len() - amount,
            state: self,
        }
    }

    pub fn fishing_facilities(mut self, amount: usize) -> FishingFacilityBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let start = self.global_data_timestamp_counter;
            let end = start + self.default_fishing_facility_duration;

            let mut facility = FishingFacility::test_default();
            facility.call_sign = None;
            facility.setup_timestamp = start;
            facility.removed_timestamp = Some(end);

            self.fishing_facilities
                .push(FishingFacilityConctructor { facility });

            self.global_data_timestamp_counter = end + self.trip_data_timestamp_gap;
        }

        FishingFacilityBuilder {
            current_index: self.fishing_facilities.len() - amount,
            state: self,
        }
    }

    pub fn manual_delivery_points(mut self, amount: usize) -> ManualDeliveryPointsBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let id =
                DeliveryPointId::try_from(format!("DP{}", self.delivery_point_id_counter)).unwrap();

            let name = format!("{}_name", id.as_ref());

            self.delivery_point_id_counter += 1;
            self.manual_delivery_points
                .push(ManualDeliveryPointConstructor {
                    val: ManualDeliveryPoint {
                        id,
                        name,
                        type_id: DeliveryPointType::Fiskemottak,
                    },
                });
        }

        ManualDeliveryPointsBuilder {
            current_index: self.manual_delivery_points.len() - amount,
            state: self,
        }
    }

    pub fn aqua_cultures(mut self, amount: usize) -> AquaCultureBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let mut val = fiskeridir_rs::AquaCultureEntry::test_default();
            val.delivery_point_id =
                DeliveryPointId::try_from(format!("DP{}", self.delivery_point_id_counter)).unwrap();
            self.delivery_point_id_counter += 1;
            self.aqua_cultures.push(AquaCultureConstructor { val });
        }

        AquaCultureBuilder {
            current_index: self.aqua_cultures.len() - amount,
            state: self,
        }
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

        self.storage
            .add_ers_dca(Box::new(self.hauls.into_iter().map(|v| Ok(v.dca))))
            .await
            .unwrap();

        self.storage
            .add_ers_tra(self.tra.into_iter().map(|v| v.tra).collect())
            .await
            .unwrap();

        self.storage
            .add_ers_dep(self.dep.into_iter().map(|v| v.dep).collect())
            .await
            .unwrap();

        self.storage
            .add_ers_por(self.por.into_iter().map(|v| v.por).collect())
            .await
            .unwrap();

        let delivery_point_ids: HashSet<DeliveryPointId> = self
            .aqua_cultures
            .iter()
            .map(|v| v.val.delivery_point_id.clone())
            .chain(self.mattilsynet.iter().map(|v| v.val.id.clone()))
            .chain(self.manual_delivery_points.iter().map(|v| v.val.id.clone()))
            .collect();

        self.storage
            .add_aqua_culture_register(self.aqua_cultures.into_iter().map(|v| v.val).collect())
            .await
            .unwrap();

        self.storage
            .add_mattilsynet_delivery_points(self.mattilsynet.into_iter().map(|v| v.val).collect())
            .await
            .unwrap();

        self.storage
            .add_manual_delivery_points(
                self.manual_delivery_points
                    .into_iter()
                    .map(|v| v.val)
                    .collect(),
            )
            .await;

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
                    let departure_timestamp = dep.departure_timestamp;
                    let arrival_timestamp = por.arrival_timestamp;
                    let start_port = dep.port.code.clone();
                    let end_port = por.port.code.clone();

                    self.storage.add_ers_dep(vec![dep]).await.unwrap();
                    self.storage.add_ers_por(vec![por]).await.unwrap();
                    if let Some(precision) = t.precision_id {
                        let mmsi = t
                            .mmsi
                            .expect("cannot add precision to trip of vessel without mmsi");
                        let start_point = match precision {
                            PrecisionId::FirstMovedPoint => TripPrecisonStartPoint::FirstPoint {
                                start: departure_timestamp,
                                mmsi,
                            },
                            PrecisionId::DeliveryPoint => {
                                panic!("cannot add precision type for ers trip")
                            }
                            PrecisionId::Port => TripPrecisonStartPoint::Port {
                                start_port: start_port
                                    .expect("cant add precision to dep without port code set"),
                                end_port: end_port
                                    .expect("cant add precision to por without port code set"),
                                start: departure_timestamp,
                                end: arrival_timestamp,
                                mmsi,
                            },
                            PrecisionId::DockPoint => TripPrecisonStartPoint::DockPoint {
                                start_port: start_port
                                    .expect("cant add precision to dep without port code set"),
                                end_port: end_port
                                    .expect("cant add precision to por without port code set"),
                                start: departure_timestamp,
                                end: arrival_timestamp,
                                mmsi,
                            },
                        };

                        let mut ais_positions =
                            add_precision_to_trip(self.storage.as_ref(), start_point).await;

                        self.ais_positions.append(&mut ais_positions);
                    }
                }
                TripSpecification::Landing {
                    start_landing,
                    end_landing,
                } => {
                    let start_landing_timestamp = start_landing.landing_timestamp;
                    let end_landing_timestamp = end_landing.landing_timestamp;
                    let delivery_point_id = end_landing.delivery_point.id.clone();
                    self.landings.append(&mut vec![start_landing, end_landing]);
                    if let Some(precision) = t.precision_id {
                        let mmsi = t
                            .mmsi
                            .expect("cannot add precision to trip of vessel without mmsi");
                        let start_point = match precision {
                            PrecisionId::FirstMovedPoint => TripPrecisonStartPoint::FirstPoint {
                                start: start_landing_timestamp,
                                mmsi,
                            },
                            PrecisionId::DeliveryPoint => TripPrecisonStartPoint::DeliveryPoint {
                                end: end_landing_timestamp,
                                mmsi,
                                id: delivery_point_id
                                    .expect("cannot add precision to trip without delivery point"),
                            },
                            PrecisionId::Port | PrecisionId::DockPoint => {
                                panic!("cannot add precision type for Landings trip")
                            }
                        };

                        let mut ais_positions =
                            add_precision_to_trip(self.storage.as_ref(), start_point).await;

                        self.ais_positions.append(&mut ais_positions);
                    }
                }
            }
        }

        self.storage
            .add_landings(Box::new(self.landings.into_iter().map(Ok)), 2023)
            .await
            .unwrap();

        self.ais_vms_positions
            .into_iter()
            .for_each(|v| match v.position {
                AisOrVmsPosition::Ais(a) => self
                    .ais_positions
                    .push(AisPositionConstructor { position: a }),
                AisOrVmsPosition::Vms(v) => self
                    .vms_positions
                    .push(VmsPositionConstructor { position: v }),
            });

        self.storage
            .add_vms(self.vms_positions.into_iter().map(|v| v.position).collect())
            .await
            .unwrap();
        self.ais_data_sender
            .send(DataMessage {
                positions: self.ais_positions.into_iter().map(|v| v.position).collect(),
                static_messages: vec![],
            })
            .unwrap();
        self.ais_data_confirmation.recv().await.unwrap();

        let mut ais_positions = self.storage.all_ais().await;
        let mut vms_positions = self.storage.all_vms().await;
        let mut ais_vms_positions = self.storage.all_ais_vms().await;

        let mut engine = self.engine.run_single().await;
        loop {
            if engine.current_state_name() == "Pending" {
                break;
            }
            engine = engine.run_single().await;
        }

        let mut vessels: Vec<Vessel> = self.storage.vessels().try_collect().await.unwrap();
        vessels.sort_by_key(|v| v.fiskeridir.id);

        let mut trips = self
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

        assert!(trips.len() < 100);

        let mut hauls = self
            .storage
            .hauls(HaulsQuery::default())
            .unwrap()
            .try_collect::<Vec<Haul>>()
            .await
            .unwrap();

        let mut dep = self.storage.all_dep().await;
        let mut por = self.storage.all_por().await;
        let all_delivery_points = self
            .storage
            .delivery_points()
            .try_collect::<Vec<DeliveryPoint>>()
            .await
            .unwrap();

        let mut delivery_points: Vec<DeliveryPoint> = all_delivery_points
            .iter()
            .filter(|v| delivery_point_ids.contains(&v.id))
            .cloned()
            .collect();

        let landings = self
            .storage
            .landings(LandingsQuery {
                sorting: Some(LandingsSorting::LandingTimestamp),
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .unwrap()
            .try_collect::<Vec<Landing>>()
            .await
            .unwrap();

        let mut fishing_facilities = self
            .storage
            .fishing_facilities(FishingFacilitiesQuery {
                mmsis: None,
                fiskeridir_vessel_ids: None,
                tool_types: None,
                active: None,
                setup_ranges: None,
                removed_ranges: None,
                ordering: None,
                sorting: None,
                pagination: Pagination::<FishingFacilities>::new(Some(100), Some(0)),
            })
            .try_collect::<Vec<FishingFacility>>()
            .await
            .unwrap();

        assert!(fishing_facilities.len() < 100);

        // We want all positions to be ordered by how they were created, we exploit the fact that
        // mmsis are an increasing counter and that msgtime is increased for each created position.
        ais_positions.sort_by_key(|v| (v.mmsi, v.msgtime));
        ais_vms_positions.sort_by_key(|v| v.timestamp);
        vms_positions.sort_by_key(|v| (v.call_sign.clone(), v.timestamp));
        trips.sort_by_key(|v| (v.fiskeridir_vessel_id, v.period.start()));
        hauls.sort_by_key(|v| (v.start_timestamp, v.haul_id));
        dep.sort_by_key(|v| (v.fiskeridir_vessel_id, v.timestamp));
        por.sort_by_key(|v| (v.fiskeridir_vessel_id, v.timestamp));
        delivery_points.sort_by_key(|v| v.id.clone());
        fishing_facilities.sort_by_key(|v| (v.fiskeridir_vessel_id, v.setup_timestamp));
        assert_eq!(vessels.len(), num_vessels);

        TestState {
            ais_positions,
            vessels,
            ais_vms_positions,
            vms_positions,
            trips,
            landings,
            hauls,
            dep,
            por,
            all_delivery_points,
            delivery_points,
            fishing_facilities,
        }
    }
}

async fn add_precision_to_trip(
    storage: &dyn TestStorage,
    start: TripPrecisonStartPoint,
) -> Vec<AisPositionConstructor> {
    match start {
        TripPrecisonStartPoint::Port {
            start_port,
            end_port,
            start,
            end,
            mmsi,
        } => {
            let start_port = storage
                .port(start_port.as_str())
                .await
                .expect("cannot add port precision to trip without start port");
            let end_port = storage
                .port(end_port.as_str())
                .await
                .expect("cannot add port precision to trip without end port");
            let start_coords = start_port
                .coordinates
                .expect("cannot add port precision on start port without coordinates");
            let end_coords = end_port
                .coordinates
                .expect("cannot add port precision on end port without coordinates");

            let mut ais_positions = Vec::with_capacity(3);

            let mut position = NewAisPosition::test_default(mmsi, start - Duration::seconds(1));
            position.latitude = start_coords.latitude;
            position.longitude = start_coords.longitude;
            ais_positions.push(AisPositionConstructor { position });

            // We need atleast a single point within trip to enable precision
            let mut position = NewAisPosition::test_default(mmsi, end - Duration::seconds(1));
            position.latitude = start_coords.latitude;
            position.longitude = start_coords.longitude;
            ais_positions.push(AisPositionConstructor { position });

            let mut position = NewAisPosition::test_default(mmsi, end + Duration::seconds(1));
            position.latitude = end_coords.latitude;
            position.longitude = end_coords.longitude;
            ais_positions.push(AisPositionConstructor { position });

            ais_positions
        }
        TripPrecisonStartPoint::DeliveryPoint { id, end, mmsi } => {
            let delivery_point = storage
                .delivery_point(&id)
                .await
                .expect("cannot add delivery point precision to non-existing delivery point");

            let latitude = delivery_point
                .latitude
                .expect("cannot add delivery point precision to delivery point without latitude");
            let longitude = delivery_point
                .longitude
                .expect("cannot add delivery point precision to delivery point without longitude");

            let mut ais_positions = Vec::with_capacity(1);
            let mut position = NewAisPosition::test_default(mmsi, end + Duration::seconds(1));
            position.latitude = latitude;
            position.longitude = longitude;
            ais_positions.push(AisPositionConstructor { position });
            ais_positions
        }
        TripPrecisonStartPoint::FirstPoint { start, mmsi } => {
            let mut ais_positions = Vec::with_capacity(20);
            for i in 0..20 {
                let mut position = NewAisPosition::test_default(mmsi, start + Duration::seconds(i));
                position.latitude = 70.0 + i as f64 * 0.01;
                position.longitude = 20.0 + i as f64 * 0.01;
                ais_positions.push(AisPositionConstructor { position });
            }
            ais_positions
        }
        TripPrecisonStartPoint::DockPoint {
            start_port,
            end_port,
            start,
            end,
            mmsi,
        } => {
            let start_dock_points = storage.dock_points_of_port(start_port.as_str()).await;
            let end_dock_points = storage.dock_points_of_port(end_port.as_str()).await;

            let start_dock_point = start_dock_points
                .first()
                .expect("cannot add dock point precision to trip without start dock points");
            let end_dock_point = end_dock_points
                .first()
                .expect("cannot add dock point precision to trip without end dock points");

            let mut ais_positions = Vec::with_capacity(3);
            let mut position = NewAisPosition::test_default(mmsi, start - Duration::seconds(1));
            position.latitude = start_dock_point.latitude;
            position.longitude = start_dock_point.longitude;
            ais_positions.push(AisPositionConstructor { position });

            // We need atleast a single point within trip to enable precision
            let mut position = NewAisPosition::test_default(mmsi, end - Duration::seconds(1));
            position.latitude = start_dock_point.latitude;
            position.longitude = end_dock_point.longitude;
            ais_positions.push(AisPositionConstructor { position });

            let mut position = NewAisPosition::test_default(mmsi, end + Duration::seconds(1));
            position.latitude = end_dock_point.latitude;
            position.longitude = end_dock_point.longitude;
            ais_positions.push(AisPositionConstructor { position });
            ais_positions
        }
    }
}
