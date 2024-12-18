use std::collections::HashSet;

use chrono::Datelike;

use crate::*;

use super::cycle::Cycle;

pub struct LandingBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct LandingVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

pub struct LandingTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct LandingDeliveryPointBuilder {
    pub state: DeliveryPointBuilder,
    pub current_index: usize,
}

#[derive(Debug, Clone)]
pub struct LandingConstructor {
    pub landing: fiskeridir_rs::Landing,
    pub cycle: Cycle,
}

impl LandingBuilder {
    /// Adds one `WeeklySale` to each unique combination of landings in the current `LandingBuilder` builder.
    pub fn weekly_sales(mut self) -> WeeklySaleLandingBuilder {
        let base = &mut self.state;

        let amount = add_weekly_sales(base, self.current_index);

        WeeklySaleLandingBuilder {
            current_index: base.weekly_sales.len() - amount,
            state: self,
        }
    }
}

impl LandingTripBuilder {
    /// Adds one `WeeklySale` to each unique combination of landings in the current `LandingTripBuilder` builder.
    pub fn weekly_sales(mut self) -> WeeklySaleLandingTripBuilder {
        let base = &mut self.state.state.state;

        let amount = add_weekly_sales(base, self.current_index);

        WeeklySaleLandingTripBuilder {
            current_index: base.weekly_sales.len() - amount,
            state: self,
        }
    }
}

fn add_weekly_sales(base: &mut TestStateBuilder, current_idx: usize) -> usize {
    let weekly_sale_ids = base.landings[current_idx..]
        .iter()
        .filter_map(|l| l.landing.vessel.length_group_code.map(|v| (v, &l.landing)))
        .map(|(length_group, l)| WeeklySaleId {
            iso_week: l.landing_timestamp.iso_week(),
            vessel_length_group: length_group,
            gear_group: l.gear.group,
            species: l.product.species.fdir_code,
            condition: l.product.condition,
            quality: l.product.quality,
        })
        .collect::<HashSet<_>>();

    let amount = weekly_sale_ids.len();

    base.weekly_sales
        .extend(weekly_sale_ids.into_iter().map(|v| WeeklySaleContructor {
            cycle: base.cycle,
            weekly_sale: WeeklySale::test_new(v),
        }));

    amount
}
