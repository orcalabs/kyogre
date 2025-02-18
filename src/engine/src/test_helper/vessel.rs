use std::str::FromStr;

use fiskeridir_rs::{
    LandingMonth, NonEmptyString, OrgId, RegisterVesselEntityType, RegisterVesselOwner,
};

use super::ais_vms::AisOrVmsPosition;
use super::landing::LandingVesselBuilder;
use super::*;
use crate::test_helper::item_distribution::ItemDistribution;
use crate::*;

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
    pub cycle: Cycle,
    pub set_engine_building_year: bool,
    pub(crate) clear_trip_precision: bool,
    pub(crate) clear_trip_distancing: bool,
    pub(crate) active_vessel: Option<bool>,
}

impl VesselBuilder {
    pub fn set_logged_in(mut self) -> Self {
        let call_sign = Some(TEST_SIGNED_IN_VESSEL_CALLSIGN.parse().unwrap());
        let vessel = &mut self.state.vessels[self.current_index];
        vessel.fiskeridir.radio_call_sign = call_sign.clone();
        vessel.ais.call_sign = call_sign;
        self
    }

    pub fn set_call_sign(mut self, call_sign: &CallSign) -> VesselBuilder {
        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        assert!(num_vessels > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.fiskeridir.radio_call_sign = Some(call_sign.clone());
            v.ais.call_sign = Some(call_sign.clone());
        }
        self
    }

    pub fn set_engine_building_year(mut self) -> VesselBuilder {
        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        assert!(num_vessels > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.set_engine_building_year = true;
        }
        self
    }

    pub fn set_org_id_of_owner(mut self, org_id: OrgId) -> VesselBuilder {
        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        assert!(num_vessels > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.fiskeridir.owners = vec![RegisterVesselOwner {
                city: None,
                entity_type: RegisterVesselEntityType::Company,
                id: Some(org_id),
                name: NonEmptyString::from_str("test").unwrap(),
                postal_code: 9000,
            }];
        }
        self
    }

    pub fn active_vessel(mut self) -> VesselBuilder {
        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        assert!(num_vessels > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.active_vessel = Some(true);
        }
        self
    }

    pub fn historic_vessel(mut self) -> VesselBuilder {
        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        assert!(num_vessels > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.active_vessel = Some(false);
        }
        self
    }
    pub fn clear_trip_distancing(mut self) -> VesselBuilder {
        let base = &mut self.state;
        let num_trips = base.vessels[self.current_index..].len();

        assert!(num_trips > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.clear_trip_distancing = true;
        }
        self
    }
    pub fn clear_trip_precision(mut self) -> VesselBuilder {
        let base = &mut self.state;
        let num_trips = base.vessels[self.current_index..].len();

        assert!(num_trips > 0);

        for v in base.vessels[self.current_index..].iter_mut() {
            v.clear_trip_precision = true;
        }
        self
    }
    pub fn fishing_facilities(mut self, amount: usize) -> FishingFacilityVesselBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter_mut().enumerate() {
            let num_facilities = distribution.num_elements(i);

            let vessel_call_sign = vessel
                .fiskeridir
                .radio_call_sign
                .as_ref()
                .expect("cannot add fishing facilites to vessel without call sign");

            for _ in 0..num_facilities {
                let start = base.global_data_timestamp_counter;
                let end = start + base.default_fishing_facility_duration;

                let mut facility = FishingFacility::test_default();
                facility.call_sign = Some(vessel_call_sign.clone());
                facility.setup_timestamp = start;
                facility.removed_timestamp = Some(end);

                base.fishing_facilities.push(FishingFacilityConctructor {
                    facility,
                    cycle: base.cycle,
                });

                base.global_data_timestamp_counter = end + base.trip_data_timestamp_gap;
            }
        }

        FishingFacilityVesselBuilder {
            current_index: base.fishing_facilities.len() - amount,
            state: self,
        }
    }
    pub fn hauls(mut self, amount: usize) -> HaulVesselBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter_mut().enumerate() {
            let num_hauls = distribution.num_elements(i);

            for _ in 0..num_hauls {
                let timestamp = base.global_data_timestamp_counter;
                let mut dca = fiskeridir_rs::ErsDca::test_default(
                    next_ers_message_id(),
                    Some(vessel.fiskeridir.id),
                );

                let start = timestamp;
                let end = timestamp + base.default_haul_duration;
                dca.message_info.set_message_timestamp(start);
                dca.set_start_timestamp(start);

                dca.set_stop_timestamp(end);

                base.hauls.push(HaulConstructor {
                    dca,
                    cycle: base.cycle,
                });

                base.global_data_timestamp_counter = end + base.data_timestamp_gap;
            }
        }

        HaulVesselBuilder {
            current_index: base.hauls.len() - amount,
            state: self,
        }
    }

    pub fn por(mut self, amount: usize) -> PorVesselBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter_mut().enumerate() {
            let num_por = distribution.num_elements(i);

            for _ in 0..num_por {
                let timestamp = base.global_data_timestamp_counter;
                let message_number = base
                    .ers_message_number_per_vessel
                    .get_mut(&vessel.key)
                    .unwrap();

                let por = fiskeridir_rs::ErsPor::test_default(
                    next_ers_message_id(),
                    vessel.fiskeridir.id,
                    timestamp,
                    *message_number,
                );
                *message_number += 1;

                base.por.push(PorConstructor {
                    por,
                    cycle: base.cycle,
                });
                base.global_data_timestamp_counter += base.data_timestamp_gap;
            }
        }
        PorVesselBuilder {
            current_index: base.por.len() - amount,
            state: self,
        }
    }
    pub fn dep(mut self, amount: usize) -> DepVesselBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter_mut().enumerate() {
            let num_dep = distribution.num_elements(i);

            for _ in 0..num_dep {
                let timestamp = base.global_data_timestamp_counter;
                let message_number = base
                    .ers_message_number_per_vessel
                    .get_mut(&vessel.key)
                    .unwrap();

                let dep = fiskeridir_rs::ErsDep::test_default(
                    next_ers_message_id(),
                    vessel.fiskeridir.id,
                    timestamp,
                    *message_number,
                );
                *message_number += 1;

                base.dep.push(DepConstructor {
                    dep,
                    cycle: base.cycle,
                });
                base.global_data_timestamp_counter += base.data_timestamp_gap;
            }
        }

        DepVesselBuilder {
            current_index: base.dep.len() - amount,
            state: self,
        }
    }
    pub fn tra(mut self, amount: usize) -> TraVesselBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter_mut().enumerate() {
            let num_tra = distribution.num_elements(i);

            for _ in 0..num_tra {
                let timestamp = base.global_data_timestamp_counter;
                let mut tra = fiskeridir_rs::ErsTra::test_default(
                    next_ers_message_id(),
                    Some(vessel.fiskeridir.id),
                    timestamp,
                );

                tra.message_info.set_message_timestamp(timestamp);

                base.tra.push(TraConstructor {
                    tra,
                    cycle: base.cycle,
                });
                base.global_data_timestamp_counter += base.data_timestamp_gap;
            }
        }

        TraVesselBuilder {
            current_index: base.tra.len() - amount,
            state: self,
        }
    }
    pub fn landings(mut self, amount: usize) -> LandingVesselBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();
        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter_mut().enumerate() {
            let num_landings = distribution.num_elements(i);

            for _ in 0..num_landings {
                let timestamp = base.global_data_timestamp_counter;
                let mut landing = fiskeridir_rs::Landing::test_default(
                    base.landing_id_counter as i64,
                    Some(vessel.fiskeridir.id),
                );

                let ts = timestamp;
                landing.landing_timestamp = ts;
                landing.landing_time = ts.time();
                landing.landing_month = LandingMonth::from(ts);

                base.landings.push(LandingConstructor {
                    cycle: base.cycle,
                    landing,
                });

                base.landing_id_counter += 1;

                base.global_data_timestamp_counter += base.data_timestamp_gap;
            }
        }

        LandingVesselBuilder {
            current_index: self.state.landings.len() - amount,
            state: self,
        }
    }

    pub fn landing_trips(mut self, amount: usize) -> TripBuilder {
        assert!(amount != 0);
        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter().enumerate() {
            let num_trips = distribution.num_elements(i);

            for _ in 0..num_trips {
                let timestamp = base.global_data_timestamp_counter;
                let start = timestamp;
                let end = timestamp + base.default_trip_duration;
                let mut start_landing = fiskeridir_rs::Landing::test_default(
                    base.landing_id_counter as i64,
                    Some(vessel.fiskeridir.id),
                );

                // FIXME: Why are we creating 4 landings here?
                start_landing.landing_timestamp = start;
                start_landing.landing_time = start.time();
                start_landing.landing_month = LandingMonth::from(start);

                base.landings.push(LandingConstructor {
                    landing: start_landing.clone(),
                    cycle: base.cycle,
                });

                base.landing_id_counter += 1;

                let mut end_landing = fiskeridir_rs::Landing::test_default(
                    base.landing_id_counter as i64,
                    Some(vessel.fiskeridir.id),
                );

                end_landing.landing_timestamp = end;
                end_landing.landing_time = end.time();
                end_landing.landing_month = LandingMonth::from(end);

                base.landings.push(LandingConstructor {
                    landing: end_landing.clone(),
                    cycle: base.cycle,
                });

                base.landing_id_counter += 1;

                base.trips.push(TripConstructor {
                    trip_specification: TripSpecification::Landing {
                        start_landing,
                        end_landing,
                    },
                    current_data_timestamp: start + Duration::seconds(1),
                    vessel_id: vessel.fiskeridir.id,
                    vessel_call_sign: vessel.fiskeridir.radio_call_sign.clone(),
                    precision_id: None,
                    mmsi: Some(vessel.ais.mmsi),
                    cycle: base.cycle,
                    current_ers_landing_data_timestamp: end + Duration::seconds(1),
                });

                base.global_data_timestamp_counter = end + base.data_timestamp_gap;
            }
        }

        TripBuilder {
            current_index: self.state.trips.len() - amount,
            state: self,
        }
    }
    pub fn trips(mut self, amount: usize) -> TripBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter().enumerate() {
            let num_trips = distribution.num_elements(i);

            for _ in 0..num_trips {
                let timestamp = base.global_data_timestamp_counter;
                let start = timestamp;
                let end = timestamp + base.default_trip_duration;

                let message_number = base
                    .ers_message_number_per_vessel
                    .get_mut(&vessel.key)
                    .unwrap();

                let dep = fiskeridir_rs::ErsDep::test_default(
                    next_ers_message_id(),
                    vessel.fiskeridir.id,
                    start,
                    *message_number,
                );
                *message_number += 1;
                let por = fiskeridir_rs::ErsPor::test_default(
                    next_ers_message_id(),
                    vessel.fiskeridir.id,
                    end,
                    *message_number,
                );
                *message_number += 1;

                base.trips.push(TripConstructor {
                    trip_specification: TripSpecification::Ers { dep, por },
                    current_data_timestamp: start + Duration::seconds(1),
                    vessel_id: vessel.fiskeridir.id,
                    vessel_call_sign: vessel.fiskeridir.radio_call_sign.clone(),
                    precision_id: None,
                    mmsi: Some(vessel.ais.mmsi),
                    cycle: base.cycle,
                    current_ers_landing_data_timestamp: end + Duration::seconds(1),
                });
                base.global_data_timestamp_counter = end + base.data_timestamp_gap;
            }
        }

        TripBuilder {
            current_index: self.state.trips.len() - amount,
            state: self,
        }
    }
    pub fn vms_positions(mut self, amount: usize) -> VmsPositionBuilder {
        assert!(amount != 0);

        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let call_sign = vessel.fiskeridir.radio_call_sign.clone().unwrap();

            for i in 0..num_positions {
                let timestamp = base.global_data_timestamp_counter;
                timestamps.push(timestamp);

                let lat = 72.12 + 0.001 * i as f64;
                let lon = 25.12 + 0.001 * i as f64;

                let mut position =
                    fiskeridir_rs::Vms::test_default(rand::random(), call_sign.clone(), timestamp);
                position.latitude = Some(lat);
                position.longitude = Some(lon);
                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(VmsPositionConstructor {
                    position,
                    cycle: base.cycle,
                });
            }

            base.vms_positions.append(&mut positions)
        }

        VmsPositionBuilder {
            current_index: base.vms_positions.len() - amount,
            state: self,
        }
    }
    pub fn ais_vms_positions(mut self, amount: usize) -> AisVmsPositionBuilder {
        assert!(amount != 0);
        let base = &mut self.state;

        let num_vessels = base.vessels[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_vessels);

        let mut index = 0;
        for (i, vessel) in base.vessels[self.current_index..].iter().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            let call_sign = vessel.fiskeridir.radio_call_sign.clone().unwrap();

            for i in 0..num_positions {
                let timestamp = base.global_data_timestamp_counter;
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
                    let mut pos = NewAisPosition::test_default(vessel.ais.mmsi, timestamp);
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
            }

            base.ais_vms_positions.append(&mut positions);
        }

        AisVmsPositionBuilder {
            current_index: base.ais_vms_positions.len() - amount,
            state: self,
        }
    }
    pub fn ais_positions(mut self, amount: usize) -> AisPositionBuilder {
        assert!(amount != 0);

        let base = &mut self.state;
        let num_vessels = base.vessels[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_vessels);

        for (i, vessel) in base.vessels[self.current_index..].iter().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);
            let mut timestamps = Vec::with_capacity(num_positions);

            for i in 0..num_positions {
                let timestamp = base.global_data_timestamp_counter;
                timestamps.push(timestamp);

                let lat = 72.12 + 0.001 * i as f64;
                let lon = 25.12 + 0.001 * i as f64;

                let mut position = NewAisPosition::test_default(vessel.ais.mmsi, timestamp);

                position.latitude = lat;
                position.longitude = lon;

                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(AisPositionConstructor {
                    position,
                    cycle: base.cycle,
                });
            }

            base.ais_positions.append(&mut positions);
        }

        AisPositionBuilder {
            current_index: base.ais_positions.len() - amount,
            state: self,
        }
    }
}
