use chrono::{DateTime, Utc};
use fiskeridir_rs::{ErsDep, ErsPor};

use super::vessel::VesselBuilder;
use crate::*;

pub struct TripBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct TripConstructor {
    pub index: usize,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub trip_specification: TripSpecification,
}

#[allow(clippy::large_enum_variant)]
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

// #[derive(PartialEq, Eq, Hash)]
// pub struct TripVesselKey {
//     pub vessel_key: VesselKey,
// }

impl TripBuilder {
    pub fn modify<F>(mut self, closure: F) -> TripBuilder
    where
        F: Fn(&mut TripConstructor),
    {
        self.state.state.trips.iter_mut().for_each(|(_, trips)| {
            for t in trips.iter_mut().filter(|v| v.index < self.current_index) {
                closure(t)
            }
        });

        self
    }

    pub fn modify_idx<F>(mut self, closure: F) -> TripBuilder
    where
        F: Fn(usize, &mut TripConstructor),
    {
        self.state.state.trips.iter_mut().for_each(|(_, trips)| {
            for t in trips.iter_mut().filter(|v| v.index < self.current_index) {
                closure(t.index, t)
            }
        });

        self
    }

    pub async fn build(self) -> TestState {
        self.state.state.build().await
    }
}
