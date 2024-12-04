use fiskeridir_rs::{AquaCultureEntry, DeliveryPointId, LandingMonth};
use kyogre_core::BuyerLocation;
use test_helper::item_distribution::ItemDistribution;

use crate::*;

use super::cycle::Cycle;

pub enum DeliveryPoint {
    Manual(ManualDeliveryPoint),
    Mattilsynet(MattilsynetDeliveryPoint),
    AquaCulture(AquaCultureEntry),
    BuyerRegister(BuyerLocation),
}

pub struct DeliveryPointBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct DeliveryPointConstructor {
    pub val: DeliveryPoint,
    pub cycle: Cycle,
}

impl DeliveryPointBuilder {
    pub async fn persist(mut self) -> TestStateBuilder {
        let mut manual = Vec::new();
        let mut mattilsynet = Vec::new();
        let mut aqua_culture = Vec::new();
        let mut buyer_register = Vec::new();

        for v in self.state.delivery_points.drain(..) {
            match v.val {
                DeliveryPoint::Manual(v) => manual.push(v),
                DeliveryPoint::Mattilsynet(v) => mattilsynet.push(v),
                DeliveryPoint::AquaCulture(v) => aqua_culture.push(v),
                DeliveryPoint::BuyerRegister(v) => buyer_register.push(v),
            }
        }

        self.state.storage.add_manual_delivery_points(manual).await;

        self.state
            .storage
            .add_mattilsynet_delivery_points(mattilsynet)
            .await
            .unwrap();

        self.state
            .storage
            .add_aqua_culture_register(aqua_culture)
            .await
            .unwrap();

        self.state
            .storage
            .add_buyer_locations(buyer_register)
            .await
            .unwrap();

        self.base()
    }

    pub fn with_delivery_point_source(mut self, source: DeliveryPointSourceId) -> Self {
        for v in &mut self.state.delivery_points[self.current_index..] {
            v.val.set_source(source);
        }
        self
    }

    pub fn landings(mut self, amount: usize) -> LandingDeliveryPointBuilder {
        assert!(amount != 0);

        let base = &mut self.state;
        let delivery_points = &mut base.delivery_points[self.current_index..];

        let distribution = ItemDistribution::new(amount, delivery_points.len());

        for (i, delivery_point) in delivery_points.iter_mut().enumerate() {
            let num_landings = distribution.num_elements(i);

            for _ in 0..num_landings {
                let mut landing =
                    fiskeridir_rs::Landing::test_default(base.landing_id_counter as i64, None);

                let ts = base.global_data_timestamp_counter;
                landing.landing_timestamp = ts;
                landing.landing_time = ts.time();
                landing.landing_month = LandingMonth::from(ts);
                landing.delivery_point.id = Some(delivery_point.val.id().clone());

                base.landings.push(LandingConstructor {
                    landing,
                    cycle: base.cycle,
                });

                base.landing_id_counter += 1;

                base.global_data_timestamp_counter += base.data_timestamp_gap;
            }
        }

        LandingDeliveryPointBuilder {
            current_index: base.landings.len() - amount,
            state: self,
        }
    }
}

impl DeliveryPoint {
    pub fn id(&self) -> &DeliveryPointId {
        match self {
            DeliveryPoint::Manual(v) => &v.id,
            DeliveryPoint::Mattilsynet(v) => &v.id,
            DeliveryPoint::AquaCulture(v) => &v.delivery_point_id,
            DeliveryPoint::BuyerRegister(v) => v.delivery_point_id.as_ref().unwrap(),
        }
    }

    pub fn set_id(&mut self, id: DeliveryPointId) {
        match self {
            DeliveryPoint::Manual(v) => v.id = id,
            DeliveryPoint::Mattilsynet(v) => v.id = id,
            DeliveryPoint::AquaCulture(v) => v.delivery_point_id = id,
            DeliveryPoint::BuyerRegister(v) => v.delivery_point_id = Some(id),
        }
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        match self {
            DeliveryPoint::Manual(v) => v.name = name.into(),
            DeliveryPoint::Mattilsynet(v) => v.name = name.into(),
            DeliveryPoint::AquaCulture(v) => v.name = name.into().parse().unwrap(),
            DeliveryPoint::BuyerRegister(v) => v.name = Some(name.into()),
        }
    }

    pub fn aqua_culture(&self) -> Option<&AquaCultureEntry> {
        match self {
            DeliveryPoint::AquaCulture(v) => Some(v),
            _ => None,
        }
    }

    pub fn mattilsynet(&self) -> Option<&MattilsynetDeliveryPoint> {
        match self {
            DeliveryPoint::Mattilsynet(v) => Some(v),
            _ => None,
        }
    }

    pub fn manual(&self) -> Option<&ManualDeliveryPoint> {
        match self {
            DeliveryPoint::Manual(v) => Some(v),
            _ => None,
        }
    }

    pub fn buyer_location(&self) -> Option<&BuyerLocation> {
        match self {
            DeliveryPoint::BuyerRegister(v) => Some(v),
            _ => None,
        }
    }

    pub fn set_source(&mut self, source: DeliveryPointSourceId) {
        let (id, name): (_, &str) = match self {
            DeliveryPoint::Manual(v) => (&v.id, &v.name),
            DeliveryPoint::Mattilsynet(v) => (&v.id, &v.name),
            DeliveryPoint::AquaCulture(v) => (&v.delivery_point_id, v.name.as_ref()),
            DeliveryPoint::BuyerRegister(v) => (
                v.delivery_point_id.as_ref().unwrap(),
                v.name.as_ref().unwrap(),
            ),
        };

        *self = match source {
            DeliveryPointSourceId::Manual => Self::Manual(ManualDeliveryPoint {
                id: id.clone(),
                name: name.into(),
                type_id: DeliveryPointType::Fiskemottak,
            }),
            DeliveryPointSourceId::Mattilsynet => {
                let mut dp = MattilsynetDeliveryPoint::test_default();
                dp.id = id.clone();
                dp.name = name.into();
                Self::Mattilsynet(dp)
            }
            DeliveryPointSourceId::AquaCultureRegister => {
                let mut dp = AquaCultureEntry::test_default();
                dp.delivery_point_id = id.clone();
                dp.name = name.parse().unwrap();
                Self::AquaCulture(dp)
            }
            DeliveryPointSourceId::BuyerRegister => {
                let mut dp = BuyerLocation::test_new(id.clone());
                dp.name = Some(name.into());
                Self::BuyerRegister(dp)
            }
        };
    }
}
