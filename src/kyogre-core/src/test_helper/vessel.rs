use fiskeridir_rs::LandingMonth;

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
}

impl VesselBuilder {
    pub fn vessels(self, amount: usize) -> VesselBuilder {
        self.state.vessels(amount)
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

                base.fishing_facilities
                    .push(FishingFacilityConctructor { facility });

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
                    base.ers_message_id_counter,
                    Some(vessel.fiskeridir.id as u64),
                );

                base.ers_message_id_counter += 1;
                let start = timestamp;
                let end = timestamp + base.default_haul_duration;
                dca.message_info.set_message_timestamp(start);
                dca.set_start_timestamp(start);

                dca.set_stop_timestamp(end);

                base.hauls.push(HaulConstructor { dca });

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
                    base.ers_message_id_counter,
                    vessel.fiskeridir.id as u64,
                    timestamp,
                    *message_number,
                );
                *message_number += 1;
                base.ers_message_id_counter += 1;

                base.por.push(PorConstructor { por });
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
                    base.ers_message_id_counter,
                    vessel.fiskeridir.id as u64,
                    timestamp,
                    *message_number,
                );
                *message_number += 1;
                base.ers_message_id_counter += 1;

                base.dep.push(DepConstructor { dep });
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
                    base.ers_message_id_counter,
                    Some(vessel.fiskeridir.id as u64),
                    timestamp,
                );

                tra.message_info.set_message_timestamp(timestamp);

                base.ers_message_id_counter += 1;

                base.tra.push(TraConstructor { tra });
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

                base.landings.push(landing);

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

                start_landing.landing_timestamp = start;
                start_landing.landing_time = start.time();
                start_landing.landing_month = LandingMonth::from(start);

                base.landings.push(start_landing.clone());

                base.landing_id_counter += 1;

                let mut end_landing = fiskeridir_rs::Landing::test_default(
                    base.landing_id_counter as i64,
                    Some(vessel.fiskeridir.id),
                );

                end_landing.landing_timestamp = end;
                end_landing.landing_time = end.time();
                end_landing.landing_month = LandingMonth::from(end);

                base.landings.push(end_landing.clone());

                base.landing_id_counter += 1;

                base.trips.push(TripConstructor {
                    trip_specification: TripSpecification::Landing {
                        start_landing,
                        end_landing,
                    },
                    current_data_timestamp: start + Duration::seconds(1),
                    vessel_id: FiskeridirVesselId(vessel.fiskeridir.id),
                    vessel_call_sign: vessel.fiskeridir.radio_call_sign.clone(),
                    precision_id: None,
                    mmsi: Some(vessel.ais.mmsi),
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
                    base.ers_message_id_counter,
                    vessel.fiskeridir.id as u64,
                    start,
                    *message_number,
                );
                *message_number += 1;
                base.ers_message_id_counter += 1;
                let por = fiskeridir_rs::ErsPor::test_default(
                    base.ers_message_id_counter,
                    vessel.fiskeridir.id as u64,
                    end,
                    *message_number,
                );
                *message_number += 1;

                base.ers_message_id_counter += 1;

                base.trips.push(TripConstructor {
                    trip_specification: TripSpecification::Ers { dep, por },
                    current_data_timestamp: start + Duration::seconds(1),
                    vessel_id: FiskeridirVesselId(vessel.fiskeridir.id),
                    vessel_call_sign: vessel.fiskeridir.radio_call_sign.clone(),
                    precision_id: None,
                    mmsi: Some(vessel.ais.mmsi),
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

            for _ in 0..num_positions {
                let timestamp = base.global_data_timestamp_counter;
                timestamps.push(timestamp);
                let position =
                    fiskeridir_rs::Vms::test_default(rand::random(), call_sign.clone(), timestamp);
                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(VmsPositionConstructor { position });
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
                let position = if (i + 1) % 2 == 0 {
                    AisOrVmsPosition::Vms(fiskeridir_rs::Vms::test_default(
                        rand::random(),
                        call_sign.clone(),
                        timestamp,
                    ))
                } else {
                    AisOrVmsPosition::Ais(NewAisPosition::test_default(vessel.ais.mmsi, timestamp))
                };
                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(AisVmsPositionConstructor { index, position });
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

            for _ in 0..num_positions {
                let timestamp = base.global_data_timestamp_counter;
                timestamps.push(timestamp);
                let position = NewAisPosition::test_default(vessel.ais.mmsi, timestamp);
                base.global_data_timestamp_counter += base.data_timestamp_gap;
                positions.push(AisPositionConstructor { position });
            }

            base.ais_positions.append(&mut positions);
        }

        AisPositionBuilder {
            current_index: base.ais_positions.len() - amount,
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
                if i >= self.current_index {
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
