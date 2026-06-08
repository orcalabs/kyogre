use super::cycle::Cycle;
use crate::{HaulTripBuilder, TestStateBuilder, TripBuilder, VesselBuilder};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear};
use kyogre_core::{BarentswatchUserId, HaulEnd, HaulStart};

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
        }
    }
}
