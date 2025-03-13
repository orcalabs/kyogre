use fiskeridir_rs::Gear;
use kyogre_core::FiskeridirVesselId;
use rand::random_range;
use std::{
    collections::{BTreeMap, HashSet},
    iter,
    ops::{Add, AddAssign, Mul, Neg, SubAssign},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// Speed at which vessel has ~85% engine load
    pub service_speeds: BTreeMap<FiskeridirVesselId, f64>,
    /// How much `service_speed` is reduced by a full cargo load
    pub cargo_weight_factor: f64,
    /// Load factor for each Gear
    pub haul_load_factors: BTreeMap<Gear, f64>,
}

#[derive(Debug, Clone)]
pub struct ParamsDelta {
    pub service_speeds: BTreeMap<FiskeridirVesselId, f64>,
    pub cargo_weight_factor: f64,
    pub haul_load_factors: BTreeMap<Gear, f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum Delta {
    ServiceSpeed((FiskeridirVesselId, f64)),
    CargoWeightFactor(f64),
    HaulLoadFactor((Gear, f64)),
}

#[derive(Debug)]
pub enum DeltaMut<'a> {
    ServiceSpeed((FiskeridirVesselId, &'a mut f64)),
    CargoWeightFactor(&'a mut f64),
    HaulLoadFactor((Gear, &'a mut f64)),
}

impl Params {
    pub fn rand(vessel_ids: &HashSet<FiskeridirVesselId>, gears: &HashSet<Gear>) -> Self {
        Self {
            service_speeds: vessel_ids
                .iter()
                .map(|v| (*v, random_range(1.0..=20.0)))
                .collect(),
            cargo_weight_factor: random_range(0.1..=1.0),
            haul_load_factors: gears
                .iter()
                .map(|v| (*v, random_range(1.0..=100.0)))
                .collect(),
        }
    }
}

impl ParamsDelta {
    pub fn new(vessel_ids: &HashSet<FiskeridirVesselId>, gears: &HashSet<Gear>) -> Self {
        Self {
            service_speeds: vessel_ids.iter().map(|v| (*v, 0.1)).collect(),
            cargo_weight_factor: 0.1,
            haul_load_factors: gears.iter().map(|v| (*v, 0.1)).collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Delta> {
        let Self {
            service_speeds,
            cargo_weight_factor,
            haul_load_factors,
        } = self;

        service_speeds
            .iter()
            .map(|(id, v)| Delta::ServiceSpeed((*id, *v)))
            .chain(iter::once(Delta::CargoWeightFactor(*cargo_weight_factor)))
            .chain(
                haul_load_factors
                    .iter()
                    .map(|(g, v)| Delta::HaulLoadFactor((*g, *v))),
            )
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = DeltaMut<'_>> {
        let Self {
            service_speeds,
            cargo_weight_factor,
            haul_load_factors,
        } = self;

        service_speeds
            .iter_mut()
            .map(|(id, v)| DeltaMut::ServiceSpeed((*id, v)))
            .chain(iter::once(DeltaMut::CargoWeightFactor(cargo_weight_factor)))
            .chain(
                haul_load_factors
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::HaulLoadFactor((*g, v))),
            )
    }
}
impl DeltaMut<'_> {
    pub fn is_zero(&self) -> bool {
        match self {
            DeltaMut::ServiceSpeed((_, v)) => **v == 0.,
            DeltaMut::CargoWeightFactor(v) => **v == 0.,
            DeltaMut::HaulLoadFactor((_, v)) => **v == 0.,
        }
    }

    pub fn set_zero(&mut self) {
        match self {
            DeltaMut::ServiceSpeed((_, v)) => **v = 0.,
            DeltaMut::CargoWeightFactor(v) => **v = 0.,
            DeltaMut::HaulLoadFactor((_, v)) => **v = 0.,
        }
    }

    pub fn value(&self) -> Delta {
        match self {
            DeltaMut::ServiceSpeed((id, v)) => Delta::ServiceSpeed((*id, **v)),
            DeltaMut::CargoWeightFactor(v) => Delta::CargoWeightFactor(**v),
            DeltaMut::HaulLoadFactor((g, v)) => Delta::HaulLoadFactor((*g, **v)),
        }
    }

    pub fn neg(&mut self) {
        match self {
            DeltaMut::ServiceSpeed((_, v)) => **v = -**v,
            DeltaMut::CargoWeightFactor(v) => **v = -**v,
            DeltaMut::HaulLoadFactor((_, v)) => **v = -**v,
        }
    }
}

impl AddAssign<&ParamsDelta> for Params {
    fn add_assign(&mut self, rhs: &ParamsDelta) {
        for d in rhs.iter() {
            *self += d;
        }
    }
}

impl Add<&ParamsDelta> for Params {
    type Output = Self;

    fn add(mut self, rhs: &ParamsDelta) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<Delta> for Params {
    fn add_assign(&mut self, rhs: Delta) {
        match rhs {
            Delta::ServiceSpeed((id, delta)) => {
                self.service_speeds
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::CargoWeightFactor(v) => {
                self.cargo_weight_factor = (self.cargo_weight_factor + v).clamp(0., 1.)
            }
            Delta::HaulLoadFactor((gear, delta)) => {
                self.haul_load_factors
                    .entry(gear)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing gear: {gear}"));
            }
        }
    }
}

impl Neg for Delta {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self * -1.
    }
}

impl SubAssign<Delta> for Params {
    fn sub_assign(&mut self, rhs: Delta) {
        *self += -rhs;
    }
}

impl Mul<f64> for Delta {
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self::Output {
        match &mut self {
            Delta::ServiceSpeed((_, v)) => *v *= rhs,
            Delta::CargoWeightFactor(v) => *v *= rhs,
            Delta::HaulLoadFactor((_, v)) => *v *= rhs,
        };
        self
    }
}
