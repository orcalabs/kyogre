use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, ErsDep, ErsPor, LandingMonth};

use super::{
    ais::AisPositionTripBuilder, ais_vms::AisVmsPositionTripBuilder, haul::HaulTripBuilder,
    vessel::VesselBuilder,
};
use crate::{
    test_helper::{
        ais::AisPositionConstructor,
        ais_vms::{AisOrVmsPosition, AisVmsPositionConstructor},
        cycle::Cycle,
        haul::HaulConstructor,
        item_distribution::ItemDistribution,
        landing::LandingTripBuilder,
    },
    *,
};

pub struct TripBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Debug)]
pub struct TripConstructor {
    pub trip_specification: TripSpecification,
    pub(crate) vessel_id: FiskeridirVesselId,
    pub(crate) vessel_call_sign: Option<CallSign>,
    pub(crate) current_data_timestamp: DateTime<Utc>,
    pub(crate) precision_id: Option<PrecisionId>,
    pub(crate) mmsi: Option<Mmsi>,
    pub cycle: Cycle,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum TripSpecification {
    Ers {
        dep: ErsDep,
        por: ErsPor,
    },
    Landing {
        start_landing: fiskeridir_rs::Landing,
        end_landing: fiskeridir_rs::Landing,
    },
}

impl TripSpecification {
    pub fn set_start(&mut self, time: DateTime<Utc>) {
        match self {
            TripSpecification::Ers { dep, por: _ } => {
                dep.set_departure_timestamp(time);
                dep.message_info.set_message_timestamp(time);
            }
            TripSpecification::Landing {
                start_landing,
                end_landing: _,
            } => {
                start_landing.landing_timestamp = time;
            }
        }
    }
    pub fn set_ports(&mut self, port_id: &str) {
        match self {
            TripSpecification::Ers { dep, por } => {
                dep.port.code = Some(port_id.parse().unwrap());
                por.port.code = Some(port_id.parse().unwrap());
            }
            TripSpecification::Landing {
                start_landing: _,
                end_landing: _,
            } => {
                panic!("cant set port of landing trip");
            }
        }
    }
    pub fn set_end(&mut self, time: DateTime<Utc>) {
        match self {
            TripSpecification::Ers { dep: _, por } => {
                por.set_arrival_timestamp(time);
                por.message_info.set_message_timestamp(time);
            }
            TripSpecification::Landing {
                start_landing: _,
                end_landing,
            } => {
                end_landing.landing_timestamp = time;
            }
        }
    }
}

impl TripConstructor {
    pub fn end(&self) -> DateTime<Utc> {
        match &self.trip_specification {
            TripSpecification::Ers { dep: _, por } => por.arrival_timestamp(),
            TripSpecification::Landing {
                start_landing: _,
                end_landing,
            } => end_landing.landing_timestamp,
        }
    }
    pub fn start(&self) -> DateTime<Utc> {
        match &self.trip_specification {
            TripSpecification::Ers { dep, por: _ } => dep.departure_timestamp(),
            TripSpecification::Landing {
                start_landing,
                end_landing: _,
            } => start_landing.landing_timestamp,
        }
    }
}
impl TripBuilder {
    pub fn precision(mut self, id: PrecisionId) -> TripBuilder {
        let base = &mut self.state.state;
        let num_trips = base.trips[self.current_index..].len();

        assert!(num_trips > 0);

        for trip in base.trips[self.current_index..].iter_mut() {
            trip.precision_id = Some(id);
        }
        self
    }

    pub fn adjacent(mut self) -> TripBuilder {
        let base = &mut self.state.state;

        let num_trips = base.trips[self.current_index..].len();
        assert!(num_trips > 1);

        let mut i = self.current_index;
        while i < num_trips {
            if i != num_trips - 1 {
                let next_start = &mut base.trips[self.current_index + i + 1].start();
                let trip = &mut base.trips[i];
                trip.trip_specification.set_end(*next_start);
            }
            i += 1;
        }

        self
    }
    pub fn fishing_facilities(mut self, amount: usize) -> FishingFacilityTripBuilder {
        assert!(amount != 0);
        let base = &mut self.state.state;

        let num_trips = base.trips[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_trips);

        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_facilities = distribution.num_elements(i);

            let vessel_call_sign = trip
                .vessel_call_sign
                .as_ref()
                .expect("cannot add fishing facilites to vessel without call sign");

            for _ in 0..num_facilities {
                let start = trip.current_data_timestamp;
                let end = trip.current_data_timestamp + base.default_fishing_facility_duration;

                let mut facility = FishingFacility::test_default();
                facility.call_sign = Some(vessel_call_sign.clone());
                facility.setup_timestamp = start;
                facility.removed_timestamp = Some(end);

                base.fishing_facilities.push(FishingFacilityConctructor {
                    facility,
                    cycle: base.cycle,
                });

                trip.current_data_timestamp = end + base.trip_data_timestamp_gap;

                if trip.current_data_timestamp >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                }
            }
        }

        FishingFacilityTripBuilder {
            current_index: base.fishing_facilities.len() - amount,
            state: self,
        }
    }
    pub fn tra(mut self, amount: usize) -> TraTripBuilder {
        assert!(amount != 0);
        let base = &mut self.state.state;

        let num_trips = base.trips[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_trips);

        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_tra = distribution.num_elements(i);

            for _ in 0..num_tra {
                let ts = trip.current_data_timestamp;

                let mut tra = fiskeridir_rs::ErsTra::test_default(
                    base.ers_message_id_counter,
                    Some(trip.vessel_id),
                    ts,
                );

                tra.message_info.set_message_timestamp(ts);

                base.ers_message_id_counter += 1;

                base.tra.push(TraConstructor {
                    tra,
                    cycle: base.cycle,
                });
                trip.current_data_timestamp += base.trip_data_timestamp_gap;

                if trip.current_data_timestamp >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                }
            }
        }

        TraTripBuilder {
            current_index: base.tra.len() - amount,
            state: self,
        }
    }
    pub fn hauls(mut self, amount: usize) -> HaulTripBuilder {
        assert!(amount != 0);
        let base = &mut self.state.state;

        let num_trips = base.trips[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_trips);

        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_hauls = distribution.num_elements(i);

            for _ in 0..num_hauls {
                let mut dca = fiskeridir_rs::ErsDca::test_default(
                    base.ers_message_id_counter,
                    Some(trip.vessel_id),
                );

                base.ers_message_id_counter += 1;
                let start = trip.current_data_timestamp;
                dca.message_info.set_message_timestamp(start);
                dca.set_start_timestamp(start);

                let end = if (start + base.default_haul_duration) >= trip.end() {
                    trip.end()
                } else {
                    start + base.default_haul_duration
                };

                dca.set_stop_timestamp(end);

                base.hauls.push(HaulConstructor {
                    dca,
                    cycle: base.cycle,
                });
                trip.current_data_timestamp = end + base.trip_data_timestamp_gap;

                if trip.current_data_timestamp >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                }
            }
        }

        HaulTripBuilder {
            current_index: base.hauls.len() - amount,
            state: self,
        }
    }
    pub fn landings(mut self, amount: usize) -> LandingTripBuilder {
        assert!(amount != 0);
        let base = &mut self.state.state;

        let num_trips = base.trips[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_trips);

        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_landings = distribution.num_elements(i);

            for _ in 0..num_landings {
                let mut landing = fiskeridir_rs::Landing::test_default(
                    base.landing_id_counter as i64,
                    Some(trip.vessel_id),
                );

                let ts = trip.current_data_timestamp;
                landing.landing_timestamp = ts;
                landing.landing_time = ts.time();
                landing.landing_month = LandingMonth::from(ts);

                base.landings.push(LandingConstructor {
                    landing,
                    cycle: base.cycle,
                });

                base.landing_id_counter += 1;

                if (trip.current_data_timestamp + base.trip_data_timestamp_gap) >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                } else {
                    trip.current_data_timestamp += base.trip_data_timestamp_gap;
                }
            }
        }

        LandingTripBuilder {
            current_index: base.landings.len() - amount,
            state: self,
        }
    }

    pub fn vms_positions(mut self, amount: usize) -> VmsPositionTripBuilder {
        assert!(amount != 0);

        let base = &mut self.state.state;
        let num_trips = base.trips[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_trips);

        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let call_sign = trip.vessel_call_sign.clone().unwrap();

            for i in 0..num_positions {
                let timestamp = trip.current_data_timestamp;
                timestamps.push(timestamp);
                let mut position =
                    fiskeridir_rs::Vms::test_default(rand::random(), call_sign.clone(), timestamp);

                position.latitude = Some(72.12 + 0.001 * i as f64);
                position.longitude = Some(25.12 + 0.001 * i as f64);

                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(VmsPositionConstructor {
                    position,
                    cycle: base.cycle,
                });

                if (trip.current_data_timestamp + base.trip_data_timestamp_gap) >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                } else {
                    trip.current_data_timestamp += base.trip_data_timestamp_gap;
                }
            }

            base.vms_positions.append(&mut positions)
        }

        VmsPositionTripBuilder {
            current_index: base.vms_positions.len() - amount,
            state: self,
        }
    }

    pub fn ais_vms_positions(mut self, amount: usize) -> AisVmsPositionTripBuilder {
        assert!(amount != 0);
        let base = &mut self.state.state;

        let num_trips = base.trips[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_trips);

        let mut index = 0;
        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let call_sign = trip.vessel_call_sign.clone().unwrap();

            for i in 0..num_positions {
                let timestamp = trip.current_data_timestamp;
                timestamps.push(timestamp);
                let lat = 72.12 + 0.001 * i as f64;
                let lon = 25.12 + 0.001 * i as f64;

                let position = if (i + 1) % 2 == 0 {
                    let mut pos = fiskeridir_rs::Vms::test_default(
                        rand::random(),
                        call_sign.clone(),
                        timestamp,
                    );
                    pos.latitude = Some(lat);
                    pos.longitude = Some(lon);
                    AisOrVmsPosition::Vms(pos)
                } else {
                    let mut pos = NewAisPosition::test_default(trip.mmsi.unwrap(), timestamp);
                    pos.latitude = lat;
                    pos.longitude = lon;
                    AisOrVmsPosition::Ais(pos)
                };
                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(AisVmsPositionConstructor {
                    index,
                    position,
                    cycle: base.cycle,
                });
                index += 1;
                if (trip.current_data_timestamp + base.trip_data_timestamp_gap) >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                } else {
                    trip.current_data_timestamp += base.trip_data_timestamp_gap;
                }
            }

            base.ais_vms_positions.append(&mut positions);
        }

        AisVmsPositionTripBuilder {
            current_index: base.ais_vms_positions.len() - amount,
            state: self,
        }
    }

    pub fn ais_positions(mut self, amount: usize) -> AisPositionTripBuilder {
        assert!(amount != 0);

        let base = &mut self.state.state;
        let num_trips = base.trips[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_trips);

        for (i, trip) in base.trips[self.current_index..].iter_mut().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            for j in 0..num_positions {
                let timestamp = trip.current_data_timestamp;
                timestamps.push(timestamp);
                let mut position = NewAisPosition::test_default(trip.mmsi.unwrap(), timestamp);
                position.latitude = 72.12 + 0.001 * j as f64;
                position.longitude = 25.12 + 0.001 * j as f64;

                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(AisPositionConstructor {
                    position,
                    cycle: base.cycle,
                });

                if (trip.current_data_timestamp + base.trip_data_timestamp_gap) >= trip.end() {
                    trip.current_data_timestamp = trip.end();
                } else {
                    trip.current_data_timestamp += base.trip_data_timestamp_gap;
                }
            }

            base.ais_positions.append(&mut positions);
        }

        AisPositionTripBuilder {
            current_index: base.ais_positions.len() - amount,
            state: self,
        }
    }
}
