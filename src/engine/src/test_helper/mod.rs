use crate::{
    AisConsumeLoop, AisPosition, AisVms, AisVmsConflict, Arrival, Cluster, DataMessage,
    DeliveryPoint, DeliveryPointType, Departure, ErsTripAssembler, FisheryEngine,
    FishingFacilities, FishingFacilitiesQuery, FishingFacility, FishingSpotPredictor,
    FishingSpotWeatherPredictor, FishingWeightPredictor, FishingWeightWeatherPredictor, Haul,
    HaulsQuery, Landing, LandingTripAssembler, LandingsQuery, LandingsSorting, ManualDeliveryPoint,
    MattilsynetDeliveryPoint, Mmsi, NewAisPosition, NewAisStatic, OceanClimate, Ordering,
    Pagination, PrecisionId, PredictionRange, ScrapeState, SharedState, SpotPredictorSettings,
    Step, TripDetailed, Trips, TripsQuery, UnrealisticSpeed, Vessel, VmsPosition, Weather,
    WeightPredictorSettings,
};

use chrono::{DateTime, Duration, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use fiskeridir_rs::{DeliveryPointId, LandingMonth};
use futures::TryStreamExt;
use kyogre_core::{
    CatchLocationId, FiskeridirVesselId, MLModel, NewVesselConflict, NewWeather, TestStorage,
    TrainingMode, TripAssembler, TripDistancer, TripPositionLayer, VesselBenchmark,
};
use machine::StateMachine;
use orca_core::PsqlSettings;
use postgres::PostgresAdapter;
use std::collections::{HashMap, HashSet};
use vessel_benchmark::WeightPerHour;

mod ais;
mod ais_vms;
mod cycle;
mod delivery_points;
mod dep;
mod fishing_facility;
mod haul;
mod item_distribution;
mod landing;
pub mod levels;
mod ocean_climate;
mod por;
mod tra;
mod trip;
mod vessel;
mod vms;
mod weather;

pub use ais::*;
pub use ais_vms::*;
pub use delivery_points::*;
pub use dep::*;
pub use fishing_facility::*;
pub use haul::*;
pub use landing::*;
pub use levels::*;
pub use ocean_climate::*;
pub use por::*;
pub use tra::*;
pub use trip::*;
pub use vessel::*;
pub use vms::*;
pub use weather::*;

use self::cycle::Cycle;

pub static FISHING_SPOT_PREDICTOR_NUM_DAYS: u32 = 2;
pub static FISHING_WEIGHT_PREDICTOR_NUM_DAYS: u32 = 2;
pub static FISHING_WEIGHT_PREDICTOR_NUM_CL: u32 = 2;

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
    pub weather: Vec<Weather>,
    pub ocean_climate: Vec<OceanClimate>,
}

pub struct TestStateBuilder {
    storage: Box<dyn TestStorage>,
    vessels: Vec<VesselContructor>,
    ais_data_sender: tokio::sync::broadcast::Sender<DataMessage>,
    ais_data_confirmation: tokio::sync::mpsc::Receiver<()>,
    vessel_id_counter: i64,
    mmsi_counter: i32,
    call_sign_counter: i32,
    global_data_timestamp_counter: DateTime<Utc>,
    data_timestamp_gap: Duration,
    ais_vms_positions: Vec<AisVmsPositionConstructor>,
    ais_static: Vec<AisVesselConstructor>,
    ais_positions: Vec<AisPositionConstructor>,
    vms_positions: Vec<VmsPositionConstructor>,
    trips: Vec<TripConstructor>,
    hauls: Vec<HaulConstructor>,
    landings: Vec<LandingConstructor>,
    tra: Vec<TraConstructor>,
    dep: Vec<DepConstructor>,
    por: Vec<PorConstructor>,
    aqua_cultures: Vec<AquaCultureConstructor>,
    mattilsynet: Vec<MattilsynetConstructor>,
    manual_delivery_points: Vec<ManualDeliveryPointConstructor>,
    fishing_facilities: Vec<FishingFacilityConctructor>,
    weather: Vec<WeatherConstructor>,
    ocean_climate: Vec<OceanClimateConstructor>,
    default_trip_duration: Duration,
    default_haul_duration: Duration,
    default_fishing_facility_duration: Duration,
    trip_data_timestamp_gap: Duration,
    ers_message_id_counter: u64,
    ers_message_number_per_vessel: HashMap<VesselKey, u32>,
    delivery_point_id_counter: u64,
    landing_id_counter: u64,
    engine: FisheryEngine,
    cycle: Cycle,
    trip_queue_reset: Option<Cycle>,
    enabled_ml_models: Vec<Box<dyn MLModel>>,
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
    DistanceToShore {
        end: DateTime<Utc>,
        start: DateTime<Utc>,
        mmsi: Mmsi,
    },
}

pub fn default_fishing_spot_weather_predictor() -> Box<dyn MLModel> {
    Box::new(FishingSpotWeatherPredictor::new(SpotPredictorSettings {
        running_in_test: true,
        test_fraction: None,
        use_gpu: false,
        training_rounds: 1,
        predict_batch_size: 53,
        catch_locations: vec![CatchLocationId::new(10, 4), CatchLocationId::new(10, 5)],
        range: PredictionRange::DaysFromStartOfYear(FISHING_SPOT_PREDICTOR_NUM_DAYS),
        training_mode: TrainingMode::Single,
    }))
}

pub fn default_fishing_spot_predictor() -> Box<dyn MLModel> {
    Box::new(FishingSpotPredictor::new(SpotPredictorSettings {
        running_in_test: true,
        training_mode: TrainingMode::Single,
        test_fraction: None,
        use_gpu: false,
        training_rounds: 1,
        predict_batch_size: 53,
        range: PredictionRange::DaysFromStartOfYear(FISHING_SPOT_PREDICTOR_NUM_DAYS),
        catch_locations: vec![CatchLocationId::new(10, 4), CatchLocationId::new(10, 5)],
    }))
}

pub fn default_fishing_weight_predictor() -> Box<dyn MLModel> {
    Box::new(FishingWeightPredictor::new(WeightPredictorSettings {
        running_in_test: true,
        use_gpu: false,
        training_rounds: 1,
        predict_batch_size: 100,
        range: PredictionRange::DaysFromStartOfYear(FISHING_WEIGHT_PREDICTOR_NUM_DAYS),
        catch_locations: vec![CatchLocationId::new(10, 4), CatchLocationId::new(10, 5)],
        training_mode: TrainingMode::Single,
        test_fraction: None,
        bycatch_percentage: None,
        majority_species_group: false,
    }))
}

pub fn default_fishing_weight_weather_predictor() -> Box<dyn MLModel> {
    Box::new(FishingWeightWeatherPredictor::new(
        WeightPredictorSettings {
            running_in_test: true,
            test_fraction: None,
            training_mode: TrainingMode::Single,
            use_gpu: false,
            training_rounds: 1,
            predict_batch_size: 100,
            range: PredictionRange::DaysFromStartOfYear(FISHING_WEIGHT_PREDICTOR_NUM_DAYS),
            catch_locations: vec![CatchLocationId::new(10, 4), CatchLocationId::new(10, 5)],
            bycatch_percentage: None,
            majority_species_group: false,
        },
    ))
}

pub async fn engine(adapter: PostgresAdapter, db_settings: &PsqlSettings) -> FisheryEngine {
    let transition_log = Box::new(machine::PostgresAdapter::new(db_settings).await.unwrap());
    let db = Box::new(adapter);
    let trip_assemblers = vec![
        Box::<LandingTripAssembler>::default() as Box<dyn TripAssembler>,
        Box::<ErsTripAssembler>::default() as Box<dyn TripAssembler>,
    ];
    let benchmarks = vec![Box::<WeightPerHour>::default() as Box<dyn VesselBenchmark>];
    let trip_distancer = Box::<AisVms>::default() as Box<dyn TripDistancer>;
    let trip_layers = vec![
        Box::<AisVmsConflict>::default() as Box<dyn TripPositionLayer>,
        Box::<UnrealisticSpeed>::default() as Box<dyn TripPositionLayer>,
        Box::<Cluster>::default() as Box<dyn TripPositionLayer>,
    ];

    let shared_state = SharedState::new(
        2,
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db.clone(),
        db,
        None,
        trip_assemblers,
        benchmarks,
        trip_distancer,
        vec![],
        trip_layers,
    );
    let step = Step::initial(ScrapeState, shared_state, transition_log);
    FisheryEngine::Scrape(step)
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
            mmsi_counter: 1,
            trip_data_timestamp_gap: Duration::hours(1),
            hauls: vec![],
            default_haul_duration: Duration::hours(1),
            tra: vec![],
            global_data_timestamp_counter: Utc.with_ymd_and_hms(2010, 2, 5, 10, 0, 0).unwrap(),
            fishing_facilities: vec![],
            weather: vec![],
            ocean_climate: vec![],
            default_fishing_facility_duration: Duration::hours(1),
            dep: vec![],
            por: vec![],
            aqua_cultures: vec![],
            mattilsynet: vec![],
            manual_delivery_points: vec![],
            cycle: Cycle::new(),
            trip_queue_reset: None,
            ais_static: vec![],
            call_sign_counter: 1,
            enabled_ml_models: vec![],
        }
    }

    pub fn add_ml_models(mut self, models: Vec<Box<dyn MLModel>>) -> TestStateBuilder {
        self.enabled_ml_models = models;
        self
    }

    pub fn add_ml_model(mut self, model: Box<dyn MLModel>) -> TestStateBuilder {
        self.enabled_ml_models.push(model);
        self
    }

    pub fn data_increment(mut self, duration: Duration) -> TestStateBuilder {
        self.data_timestamp_gap = duration;
        self
    }

    pub fn data_start(mut self, time: DateTime<Utc>) -> TestStateBuilder {
        self.global_data_timestamp_counter = time;
        self
    }

    pub fn queue_trip_reset(mut self) -> TestStateBuilder {
        self.trip_queue_reset = Some(self.cycle);
        self
    }

    pub fn new_cycle(mut self) -> TestStateBuilder {
        self.cycle.increment();
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
            self.mattilsynet.push(MattilsynetConstructor {
                val,
                cycle: self.cycle,
            });
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

            self.fishing_facilities.push(FishingFacilityConctructor {
                facility,
                cycle: self.cycle,
            });

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
                    cycle: self.cycle,
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
            self.aqua_cultures.push(AquaCultureConstructor {
                val,
                cycle: self.cycle,
            });
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

            self.landings.push(LandingConstructor {
                landing,
                cycle: self.cycle,
            });

            self.landing_id_counter += 1;

            self.global_data_timestamp_counter += self.data_timestamp_gap;
        }

        LandingBuilder {
            current_index: self.landings.len() - amount,
            state: self,
        }
    }

    pub fn ais_vessels(mut self, amount: usize) -> AisVesselBuilder {
        assert!(amount != 0);

        for _ in 0..amount {
            let timestamp = self.global_data_timestamp_counter;
            let call_sign = CallSign::try_from(format!("CS{}", self.vessel_id_counter)).unwrap();
            let mut ais_static =
                NewAisStatic::test_default(Mmsi::test_new(self.mmsi_counter), call_sign.as_ref());
            ais_static.msgtime = timestamp;

            self.ais_static.push(AisVesselConstructor {
                vessel: ais_static,
                cycle: self.cycle,
            });
            self.global_data_timestamp_counter += self.data_timestamp_gap;

            self.mmsi_counter += 1;
            self.call_sign_counter += 1;
        }

        AisVesselBuilder {
            current_index: self.ais_static.len() - amount,
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

            self.tra.push(TraConstructor {
                tra,
                cycle: self.cycle,
            });
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

            self.hauls.push(HaulConstructor {
                dca,
                cycle: self.cycle,
            });

            self.global_data_timestamp_counter = end + self.data_timestamp_gap;
        }

        HaulBuilder {
            current_index: self.hauls.len() - amount,
            state: self,
        }
    }

    pub fn weather(mut self, amount: usize) -> WeatherBuilder {
        assert_ne!(amount, 0);

        for _ in 0..amount {
            let weather = NewWeather::test_default(self.global_data_timestamp_counter);
            self.weather.push(WeatherConstructor {
                weather,
                cycle: self.cycle,
            });
            self.global_data_timestamp_counter += self.data_timestamp_gap;
        }

        WeatherBuilder {
            current_index: self.weather.len() - amount,
            state: self,
        }
    }

    pub fn vessels(mut self, amount: usize) -> VesselBuilder {
        let num_vessels = self.vessels.len();
        for i in 0..amount {
            let vessel_id = FiskeridirVesselId::test_new(self.vessel_id_counter);

            let mut vessel = fiskeridir_rs::RegisterVessel::test_default(vessel_id);
            let call_sign = CallSign::try_from(format!("CS{}", self.call_sign_counter)).unwrap();
            let ais_static =
                NewAisStatic::test_default(Mmsi::test_new(self.mmsi_counter), call_sign.as_ref());
            vessel.radio_call_sign = Some(call_sign.clone());

            let key = VesselKey {
                vessel_vec_index: num_vessels + i,
            };
            self.vessels.push(VesselContructor {
                key,
                fiskeridir: vessel,
                ais: ais_static,
                cycle: self.cycle,
                clear_trip_precision: false,
                clear_trip_distancing: false,
                conflict_winner: false,
                conflict_loser: false,
            });

            self.ers_message_number_per_vessel.insert(key, 1);

            self.vessel_id_counter += 1;
            self.mmsi_counter += 1;
            self.call_sign_counter += 1;
        }

        VesselBuilder {
            current_index: self.vessels.len() - amount,
            state: self,
        }
    }

    pub async fn build(mut self) -> TestState {
        // TODO: get weather/climate from db and not conversion.
        let mut weather = Vec::new();
        let mut ocean_climate = Vec::new();

        let mut delivery_point_ids: HashSet<DeliveryPointId> = HashSet::new();

        self.engine.add_ml_models(self.enabled_ml_models);

        // TODO: dont clone in cycles
        // Use this (https://github.com/rust-lang/rust/issues/43244) if it ever merges
        for i in 1..=self.cycle.val() {
            if let Some(reset_cycle) = self.trip_queue_reset {
                if reset_cycle == i {
                    self.storage.queue_trip_reset().await;
                }
            }

            self.ais_data_sender
                .send(DataMessage {
                    positions: vec![],
                    static_messages: self
                        .vessels
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.ais.clone())
                            } else {
                                None
                            }
                        })
                        .chain(self.ais_static.iter().filter_map(|v| {
                            if v.cycle == i {
                                Some(v.vessel.clone())
                            } else {
                                None
                            }
                        }))
                        .collect(),
                })
                .unwrap();

            self.ais_data_confirmation.recv().await.unwrap();

            let conflict_overrides: Vec<NewVesselConflict> = self
                .vessels
                .iter()
                .filter_map(|v| {
                    if v.cycle == i && v.conflict_winner {
                        Some(NewVesselConflict {
                            vessel_id: v.fiskeridir.id,
                            call_sign: Some(v.fiskeridir.radio_call_sign.clone().unwrap()),
                            mmsi: Some(v.ais.mmsi),
                        })
                    } else if v.cycle == i && v.conflict_loser {
                        Some(NewVesselConflict {
                            vessel_id: v.fiskeridir.id,
                            call_sign: None,
                            mmsi: None,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            self.storage
                .manual_vessel_conflict_override(conflict_overrides)
                .await;

            self.storage
                .add_register_vessels(
                    self.vessels
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.fiskeridir.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
                .await
                .unwrap();

            self.storage
                .add_ers_dca(Box::new(
                    self.hauls
                        .clone()
                        .into_iter()
                        .filter_map(move |v| (v.cycle == i).then_some(Ok(v.dca))),
                ))
                .await
                .unwrap();

            self.storage
                .add_ers_tra(Box::new(
                    self.tra
                        .clone()
                        .into_iter()
                        .filter_map(move |v| (v.cycle == i).then_some(Ok(v.tra))),
                ))
                .await
                .unwrap();

            self.storage
                .add_ers_dep(Box::new(
                    self.dep
                        .clone()
                        .into_iter()
                        .filter_map(move |v| (v.cycle == i).then_some(Ok(v.dep))),
                ))
                .await
                .unwrap();
            self.storage
                .add_ers_por(Box::new(
                    self.por
                        .clone()
                        .into_iter()
                        .filter_map(move |v| (v.cycle == i).then_some(Ok(v.por))),
                ))
                .await
                .unwrap();

            delivery_point_ids.extend(
                self.aqua_cultures
                    .iter()
                    .map(|v| v.val.delivery_point_id.clone())
                    .chain(self.mattilsynet.iter().map(|v| v.val.id.clone()))
                    .chain(self.manual_delivery_points.iter().map(|v| v.val.id.clone())),
            );

            self.storage
                .add_aqua_culture_register(
                    self.aqua_cultures
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.val.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
                .await
                .unwrap();

            self.storage
                .add_mattilsynet_delivery_points(
                    self.mattilsynet
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.val.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
                .await
                .unwrap();

            self.storage
                .add_manual_delivery_points(
                    self.manual_delivery_points
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.val.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
                .await;

            self.storage
                .add_fishing_facilities(
                    self.fishing_facilities
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.facility.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
                .await
                .unwrap();

            for t in self.trips.iter() {
                if t.cycle != i {
                    continue;
                }
                match &t.trip_specification {
                    TripSpecification::Ers { dep, por } => {
                        let departure_timestamp = dep.departure_timestamp();
                        let arrival_timestamp = por.arrival_timestamp();
                        let start_port = dep.port.code.clone();
                        let end_port = por.port.code.clone();

                        self.storage
                            .add_ers_dep(Box::new(vec![Ok(dep.clone())].into_iter()))
                            .await
                            .unwrap();
                        self.storage
                            .add_ers_por(Box::new(vec![Ok(por.clone())].into_iter()))
                            .await
                            .unwrap();

                        if let Some(precision) = t.precision_id {
                            let mmsi = t
                                .mmsi
                                .expect("cannot add precision to trip of vessel without mmsi");
                            let start_point = match precision {
                                PrecisionId::FirstMovedPoint => {
                                    TripPrecisonStartPoint::FirstPoint {
                                        start: departure_timestamp,
                                        mmsi,
                                    }
                                }
                                PrecisionId::DeliveryPoint => {
                                    panic!("cannot add precision type for ers trip")
                                }
                                PrecisionId::Port => TripPrecisonStartPoint::Port {
                                    start_port: start_port
                                        .expect("cant add precision to dep without port code set")
                                        .into_inner(),
                                    end_port: end_port
                                        .expect("cant add precision to por without port code set")
                                        .into_inner(),
                                    start: departure_timestamp,
                                    end: arrival_timestamp,
                                    mmsi,
                                },
                                PrecisionId::DockPoint => TripPrecisonStartPoint::DockPoint {
                                    start_port: start_port
                                        .expect("cant add precision to dep without port code set")
                                        .into_inner(),
                                    end_port: end_port
                                        .expect("cant add precision to por without port code set")
                                        .into_inner(),
                                    start: departure_timestamp,
                                    end: arrival_timestamp,
                                    mmsi,
                                },
                                PrecisionId::DistanceToShore => {
                                    TripPrecisonStartPoint::DistanceToShore {
                                        end: arrival_timestamp,
                                        start: departure_timestamp,
                                        mmsi,
                                    }
                                }
                            };

                            let mut ais_positions =
                                add_precision_to_trip(self.storage.as_ref(), start_point, t.cycle)
                                    .await;

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
                        self.landings.append(&mut vec![
                            LandingConstructor {
                                landing: start_landing.clone(),
                                cycle: t.cycle,
                            },
                            LandingConstructor {
                                landing: end_landing.clone(),
                                cycle: t.cycle,
                            },
                        ]);
                        if let Some(precision) = t.precision_id {
                            let mmsi = t
                                .mmsi
                                .expect("cannot add precision to trip of vessel without mmsi");
                            let start_point = match precision {
                                PrecisionId::FirstMovedPoint => {
                                    TripPrecisonStartPoint::FirstPoint {
                                        start: start_landing_timestamp,
                                        mmsi,
                                    }
                                }
                                PrecisionId::DeliveryPoint => {
                                    TripPrecisonStartPoint::DeliveryPoint {
                                        end: end_landing_timestamp,
                                        mmsi,
                                        id: delivery_point_id.expect(
                                            "cannot add precision to trip without delivery point",
                                        ),
                                    }
                                }
                                PrecisionId::DistanceToShore => {
                                    TripPrecisonStartPoint::DistanceToShore {
                                        end: end_landing_timestamp,
                                        start: start_landing_timestamp,
                                        mmsi,
                                    }
                                }
                                PrecisionId::Port | PrecisionId::DockPoint => {
                                    panic!("cannot add precision type for Landings trip")
                                }
                            };

                            let mut ais_positions =
                                add_precision_to_trip(self.storage.as_ref(), start_point, t.cycle)
                                    .await;

                            self.ais_positions.append(&mut ais_positions);
                        }
                    }
                }
            }

            let landings = self
                .landings
                .clone()
                .into_iter()
                .filter_map(move |v| (v.cycle == i).then_some(Ok(v.landing)))
                .collect::<Vec<_>>();

            if !landings.is_empty() {
                self.storage
                    .add_landings(Box::new(landings.into_iter()), i as u32)
                    .await
                    .unwrap();
            }

            self.ais_vms_positions.iter().for_each(|v| {
                if v.cycle == i {
                    match &v.position {
                        AisOrVmsPosition::Ais(a) => {
                            self.ais_positions.push(AisPositionConstructor {
                                position: a.clone(),
                                cycle: v.cycle,
                            })
                        }
                        AisOrVmsPosition::Vms(a) => {
                            self.vms_positions.push(VmsPositionConstructor {
                                position: a.clone(),
                                cycle: v.cycle,
                            })
                        }
                    }
                }
            });

            self.storage
                .add_vms(
                    self.vms_positions
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.position.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
                .await
                .unwrap();

            self.ais_data_sender
                .send(DataMessage {
                    positions: self
                        .ais_positions
                        .iter()
                        .filter_map(|v| {
                            if v.cycle == i {
                                Some(v.position.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                    static_messages: vec![],
                })
                .unwrap();

            self.ais_data_confirmation.recv().await.unwrap();

            let new_ocean_climate = self
                .ocean_climate
                .iter()
                .filter_map(|w| {
                    if w.cycle == i {
                        Some(w.ocean_climate.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            ocean_climate.extend(new_ocean_climate.iter().map(OceanClimate::from));

            self.storage
                .add_ocean_climate(new_ocean_climate)
                .await
                .unwrap();

            let new_weather = self
                .weather
                .iter()
                .filter_map(|w| {
                    if w.cycle == i {
                        Some(w.weather.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            weather.extend(new_weather.iter().map(Weather::from));
            self.storage.add_weather(new_weather).await.unwrap();

            self.engine = self.engine.run_single().await;
            loop {
                if self.engine.current_state_name() == "Pending" {
                    break;
                }
                self.engine = self.engine.run_single().await;
            }

            for v in self.vessels.iter().filter(|v| v.cycle == i) {
                if v.clear_trip_precision {
                    self.storage.clear_trip_precision(v.fiskeridir.id).await;
                }
                if v.clear_trip_distancing {
                    self.storage.clear_trip_distancing(v.fiskeridir.id).await;
                }
            }
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

        let mut ais_positions = self.storage.all_ais().await;
        let mut vms_positions = self.storage.all_vms().await;
        let mut ais_vms_positions = self.storage.all_ais_vms().await;

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
            weather,
            ocean_climate,
        }
    }
}

async fn add_precision_to_trip(
    storage: &dyn TestStorage,
    start: TripPrecisonStartPoint,
    cycle: Cycle,
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
            ais_positions.push(AisPositionConstructor { position, cycle });

            // We need atleast a single point within trip to enable precision
            let mut position = NewAisPosition::test_default(mmsi, end - Duration::seconds(1));
            position.latitude = start_coords.latitude;
            position.longitude = start_coords.longitude;
            ais_positions.push(AisPositionConstructor { position, cycle });

            let mut position = NewAisPosition::test_default(mmsi, end + Duration::seconds(1));
            position.latitude = end_coords.latitude;
            position.longitude = end_coords.longitude;
            ais_positions.push(AisPositionConstructor { position, cycle });

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
            ais_positions.push(AisPositionConstructor { position, cycle });
            ais_positions
        }
        TripPrecisonStartPoint::FirstPoint { start, mmsi } => {
            let mut ais_positions = Vec::with_capacity(20);
            for i in 0..20 {
                let mut position = NewAisPosition::test_default(mmsi, start + Duration::seconds(i));
                position.latitude = 70.0 + i as f64 * 0.01;
                position.longitude = 20.0 + i as f64 * 0.01;
                ais_positions.push(AisPositionConstructor { position, cycle });
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
            ais_positions.push(AisPositionConstructor { position, cycle });

            // We need atleast a single point within trip to enable precision
            let mut position = NewAisPosition::test_default(mmsi, end - Duration::seconds(1));
            position.latitude = start_dock_point.latitude;
            position.longitude = start_dock_point.longitude;
            ais_positions.push(AisPositionConstructor { position, cycle });

            let mut position = NewAisPosition::test_default(mmsi, end + Duration::seconds(1));
            position.latitude = end_dock_point.latitude;
            position.longitude = end_dock_point.longitude;
            ais_positions.push(AisPositionConstructor { position, cycle });
            ais_positions
        }
        TripPrecisonStartPoint::DistanceToShore { start, end, mmsi } => {
            let mut ais_positions = Vec::with_capacity(3);
            let mut position = NewAisPosition::test_default(mmsi, start - Duration::seconds(1));
            // Close to shore
            position.latitude = 69.126682;
            position.longitude = 15.551766;
            position.speed_over_ground = Some(0.);
            ais_positions.push(AisPositionConstructor { position, cycle });

            // We need atleast a single point within trip to enable precision
            let mut position = NewAisPosition::test_default(mmsi, end - Duration::seconds(1));
            // Far from shore
            position.latitude = 72.166153;
            position.longitude = 4.474086;
            position.speed_over_ground = Some(1000.);
            ais_positions.push(AisPositionConstructor { position, cycle });

            let mut position = NewAisPosition::test_default(mmsi, end + Duration::seconds(1));
            // Close to shore
            position.latitude = 61.867577;
            position.longitude = 4.841976;
            position.speed_over_ground = Some(0.);
            ais_positions.push(AisPositionConstructor { position, cycle });
            ais_positions
        }
    }
}
