use chrono::IsoWeek;
use fiskeridir_rs::{Condition, GearGroup, Quality, VesselLengthGroup};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WeeklySaleId {
    pub iso_week: IsoWeek,
    pub vessel_length_group: VesselLengthGroup,
    pub gear_group: GearGroup,
    pub species: u32,
    pub condition: Condition,
    pub quality: Quality,
}

#[derive(Debug, Clone)]
pub struct WeeklySale {
    pub id: WeeklySaleId,
    pub sum_net_quantity_kg: f64,
    pub sum_calculated_living_weight: f64,
    pub sum_price: f64,
}

#[cfg(feature = "test")]
mod test {
    use chrono::{DateTime, Datelike, Utc};
    use rand::random;

    use super::*;

    impl WeeklySaleId {
        pub fn test_new(ts: DateTime<Utc>) -> Self {
            Self {
                iso_week: ts.iso_week(),
                vessel_length_group: VesselLengthGroup::FifteenToTwentyOne,
                gear_group: GearGroup::Trawl,
                species: 1032,
                condition: Condition::Levende,
                quality: Quality::Superior,
            }
        }
    }

    impl WeeklySale {
        pub fn test_new(id: WeeklySaleId) -> Self {
            let kg = random::<f64>() * 10_000.;
            Self {
                id,
                sum_net_quantity_kg: kg,
                sum_calculated_living_weight: kg * 0.9,
                sum_price: kg * 150.,
            }
        }
    }
}
