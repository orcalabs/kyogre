use super::cycle::Cycle;
use crate::{
    AisPositionConstructor, AisPositionUserHaulHaulTripBuilder, AisPositionUserHaulTripBuilder,
    AisPositionUserHaulVesselBuilder, HaulTripBuilder, TestStateBuilder, TripBuilder,
    VesselBuilder, test_helper::item_distribution::ItemDistribution,
};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::{CallSign, Gear};
use kyogre_core::{BarentswatchUserId, HaulEnd, HaulStart, Mmsi, NewAisPosition};

pub struct UserHaulBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct UserHaulHaulTripBuilder {
    pub state: HaulTripBuilder,
    pub current_index: usize,
}

pub struct UserHaulTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct UserHaulVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone, Debug)]
pub struct UserHaulConstructor {
    pub start: HaulStart,
    pub end: HaulEnd,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub gear: Gear,
    pub user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub cycle: Cycle,
    pub mmsi: Option<Mmsi>,
}

impl UserHaulConstructor {
    pub fn new(
        cycle: Cycle,
        start_ts: DateTime<Utc>,
        end_ts: DateTime<Utc>,
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Self {
        Self {
            start_ts,
            gear: Gear::TripleTrawl,
            start: HaulStart::test_default(),
            end: HaulEnd::test_default(),
            user_id,
            call_sign: call_sign.clone(),
            cycle,
            end_ts,
            mmsi: None,
        }
    }
}

impl UserHaulVesselBuilder {
    pub fn ais_positions(mut self, amount: usize) -> AisPositionUserHaulVesselBuilder {
        assert!(amount != 0);

        let base = &mut self.state.state;
        let num_user_hauls = base.user_hauls[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_user_hauls);

        for (i, user_haul) in base.user_hauls[self.current_index..].iter().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);

            let mut current = user_haul.start_ts;
            for i in 0..num_positions {
                let lat = 72.12 + 0.001 * i as f64;
                let lon = 25.12 + 0.001 * i as f64;

                let mut position = NewAisPosition::test_default(user_haul.mmsi.unwrap(), current);

                position.latitude = lat;
                position.longitude = lon;

                positions.push(AisPositionConstructor {
                    position,
                    cycle: base.cycle,
                });
                current += Duration::seconds(1);
            }

            base.ais_positions.append(&mut positions);
        }

        AisPositionUserHaulVesselBuilder {
            current_index: base.ais_positions.len() - amount,
            state: self,
        }
    }
}

impl UserHaulTripBuilder {
    pub fn ais_positions(mut self, amount: usize) -> AisPositionUserHaulTripBuilder {
        assert!(amount != 0);

        let base = &mut self.state.state.state;
        let num_user_hauls = base.user_hauls[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_user_hauls);

        for (i, user_haul) in base.user_hauls[self.current_index..].iter().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);

            let mut current = user_haul.start_ts;
            for i in 0..num_positions {
                let lat = 72.12 + 0.001 * i as f64;
                let lon = 25.12 + 0.001 * i as f64;

                let mut position = NewAisPosition::test_default(user_haul.mmsi.unwrap(), current);

                position.latitude = lat;
                position.longitude = lon;

                positions.push(AisPositionConstructor {
                    position,
                    cycle: base.cycle,
                });
                current += Duration::seconds(1);
            }

            base.ais_positions.append(&mut positions);
        }

        AisPositionUserHaulTripBuilder {
            current_index: base.ais_positions.len() - amount,
            state: self,
        }
    }
}

impl UserHaulHaulTripBuilder {
    pub fn ais_positions(mut self, amount: usize) -> AisPositionUserHaulHaulTripBuilder {
        assert!(amount != 0);

        let base = &mut self.state.state.state.state;
        let num_user_hauls = base.user_hauls[self.current_index..].len();

        let distribution = ItemDistribution::new(amount, num_user_hauls);

        for (i, user_haul) in base.user_hauls[self.current_index..].iter().enumerate() {
            let num_positions = distribution.num_elements(i);

            let mut positions = Vec::with_capacity(num_positions);

            let mut current = user_haul.start_ts;
            for i in 0..num_positions {
                let lat = 72.12 + 0.001 * i as f64;
                let lon = 25.12 + 0.001 * i as f64;

                let mut position = NewAisPosition::test_default(user_haul.mmsi.unwrap(), current);

                position.latitude = lat;
                position.longitude = lon;

                positions.push(AisPositionConstructor {
                    position,
                    cycle: base.cycle,
                });
                current += Duration::seconds(1);
            }

            base.ais_positions.append(&mut positions);
        }

        AisPositionUserHaulHaulTripBuilder {
            current_index: base.ais_positions.len() - amount,
            state: self,
        }
    }
}
