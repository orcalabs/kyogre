use fiskeridir_rs::Gear;
use kyogre_core::FiskeridirVesselId;
use processors::ScrewType;
use rand::random_range;
use std::{
    collections::{BTreeMap, HashSet},
    ops::{Add, AddAssign, Mul, Neg, SubAssign},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub haul_load_factors: BTreeMap<Gear, f64>,
    pub propellor_diameter: BTreeMap<FiskeridirVesselId, f64>,
    pub prismatic_coefficient: BTreeMap<FiskeridirVesselId, f64>,
    pub block_coefficent: BTreeMap<FiskeridirVesselId, f64>,
    pub propellor_efficency: BTreeMap<FiskeridirVesselId, f64>,
    pub shaft_efficiency: BTreeMap<FiskeridirVesselId, f64>,
    pub midship_section_coefficient: BTreeMap<FiskeridirVesselId, f64>,
    pub stern_parameter: BTreeMap<FiskeridirVesselId, f64>,
    pub screw_type: ScrewType,
}

#[derive(Debug, Clone)]
pub struct ParamsDelta {
    pub haul_load_factors: BTreeMap<Gear, f64>,
    pub propellor_diameter: BTreeMap<FiskeridirVesselId, f64>,
    pub prismatic_coefficient: BTreeMap<FiskeridirVesselId, f64>,
    pub block_coefficent: BTreeMap<FiskeridirVesselId, f64>,
    pub propellor_efficency: BTreeMap<FiskeridirVesselId, f64>,
    pub shaft_efficiency: BTreeMap<FiskeridirVesselId, f64>,
    pub midship_section_coefficient: BTreeMap<FiskeridirVesselId, f64>,
    pub stern_parameter: BTreeMap<FiskeridirVesselId, f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum Delta {
    HaulLoadFactor((Gear, f64)),
    PropellorDiameter((FiskeridirVesselId, f64)),
    PrismaticCoefficient((FiskeridirVesselId, f64)),
    BlockCoefficent((FiskeridirVesselId, f64)),
    PropellorEfficency((FiskeridirVesselId, f64)),
    ShaftEfficiency((FiskeridirVesselId, f64)),
    MidshipSectionCoefficient((FiskeridirVesselId, f64)),
    SternParameter((FiskeridirVesselId, f64)),
}

#[derive(Debug)]
pub enum DeltaMut<'a> {
    HaulLoadFactor((Gear, &'a mut f64)),
    PropellorDiameter((FiskeridirVesselId, &'a mut f64)),
    PrismaticCoefficient((FiskeridirVesselId, &'a mut f64)),
    BlockCoefficent((FiskeridirVesselId, &'a mut f64)),
    PropellorEfficency((FiskeridirVesselId, &'a mut f64)),
    ShaftEfficiency((FiskeridirVesselId, &'a mut f64)),
    MidshipSectionCoefficient((FiskeridirVesselId, &'a mut f64)),
    SternParameter((FiskeridirVesselId, &'a mut f64)),
}

impl Params {
    pub fn rand(
        vessel_ids: &HashSet<FiskeridirVesselId>,
        gears: &HashSet<Gear>,
        screw_type: ScrewType,
    ) -> Self {
        Self {
            haul_load_factors: gears
                .iter()
                .map(|v| (*v, random_range(1.0..=100.0)))
                .collect(),
            propellor_diameter: vessel_ids
                .iter()
                .map(|v| (*v, random_range(0.35..=0.45)))
                .collect(),
            prismatic_coefficient: vessel_ids
                .iter()
                .map(|v| (*v, random_range(0.45..=0.55)))
                .collect(),
            propellor_efficency: vessel_ids
                .iter()
                .map(|v| (*v, random_range(0.68..=0.75)))
                .collect(),
            shaft_efficiency: vessel_ids
                .iter()
                .map(|v| (*v, random_range(0.9..=0.98)))
                .collect(),
            midship_section_coefficient: vessel_ids
                .iter()
                .map(|v| (*v, random_range(0.9..=0.98)))
                .collect(),
            block_coefficent: vessel_ids
                .iter()
                .map(|v| (*v, random_range(0.55..=0.6)))
                .collect(),
            stern_parameter: vessel_ids
                .iter()
                .map(|v| (*v, random_range(8.0..=12.0)))
                .collect(),
            screw_type,
        }
    }
}

impl ParamsDelta {
    pub fn new(vessel_ids: &HashSet<FiskeridirVesselId>, gears: &HashSet<Gear>) -> Self {
        let delta: BTreeMap<FiskeridirVesselId, f64> =
            vessel_ids.iter().map(|v| (*v, 0.01)).collect();
        let stern_delta: BTreeMap<FiskeridirVesselId, f64> =
            vessel_ids.iter().map(|v| (*v, 0.1)).collect();
        Self {
            haul_load_factors: gears.iter().map(|v| (*v, 0.1)).collect(),
            propellor_diameter: delta.clone(),
            prismatic_coefficient: delta.clone(),
            block_coefficent: delta.clone(),
            propellor_efficency: delta.clone(),
            shaft_efficiency: delta.clone(),
            midship_section_coefficient: delta,
            stern_parameter: stern_delta,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Delta> {
        let Self {
            haul_load_factors,
            propellor_diameter,
            prismatic_coefficient,
            block_coefficent,
            propellor_efficency,
            shaft_efficiency,
            midship_section_coefficient,
            stern_parameter,
        } = self;

        haul_load_factors
            .iter()
            .map(|(id, v)| Delta::HaulLoadFactor((*id, *v)))
            .chain(
                propellor_diameter
                    .iter()
                    .map(|(g, v)| Delta::PropellorDiameter((*g, *v))),
            )
            .chain(
                prismatic_coefficient
                    .iter()
                    .map(|(g, v)| Delta::PrismaticCoefficient((*g, *v))),
            )
            .chain(
                block_coefficent
                    .iter()
                    .map(|(g, v)| Delta::BlockCoefficent((*g, *v))),
            )
            .chain(
                propellor_efficency
                    .iter()
                    .map(|(g, v)| Delta::PropellorEfficency((*g, *v))),
            )
            .chain(
                shaft_efficiency
                    .iter()
                    .map(|(g, v)| Delta::ShaftEfficiency((*g, *v))),
            )
            .chain(
                midship_section_coefficient
                    .iter()
                    .map(|(g, v)| Delta::MidshipSectionCoefficient((*g, *v))),
            )
            .chain(
                stern_parameter
                    .iter()
                    .map(|(g, v)| Delta::SternParameter((*g, *v))),
            )
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = DeltaMut<'_>> {
        let Self {
            haul_load_factors,
            propellor_diameter,
            prismatic_coefficient,
            block_coefficent,
            propellor_efficency,
            shaft_efficiency,
            midship_section_coefficient,
            stern_parameter,
        } = self;

        haul_load_factors
            .iter_mut()
            .map(|(id, v)| DeltaMut::HaulLoadFactor((*id, v)))
            .chain(
                propellor_diameter
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::PropellorDiameter((*g, v))),
            )
            .chain(
                prismatic_coefficient
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::PrismaticCoefficient((*g, v))),
            )
            .chain(
                block_coefficent
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::BlockCoefficent((*g, v))),
            )
            .chain(
                propellor_efficency
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::PropellorEfficency((*g, v))),
            )
            .chain(
                shaft_efficiency
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::ShaftEfficiency((*g, v))),
            )
            .chain(
                midship_section_coefficient
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::MidshipSectionCoefficient((*g, v))),
            )
            .chain(
                stern_parameter
                    .iter_mut()
                    .map(|(g, v)| DeltaMut::SternParameter((*g, v))),
            )
    }
}
impl DeltaMut<'_> {
    pub fn is_zero(&self) -> bool {
        match self {
            DeltaMut::HaulLoadFactor((_, v)) => **v == 0.,
            DeltaMut::PropellorDiameter((_, v)) => **v == 0.,
            DeltaMut::PrismaticCoefficient((_, v)) => **v == 0.,
            DeltaMut::BlockCoefficent((_, v)) => **v == 0.,
            DeltaMut::PropellorEfficency((_, v)) => **v == 0.,
            DeltaMut::ShaftEfficiency((_, v)) => **v == 0.,
            DeltaMut::MidshipSectionCoefficient((_, v)) => **v == 0.,
            DeltaMut::SternParameter((_, v)) => **v == 0.,
        }
    }

    pub fn set_zero(&mut self) {
        match self {
            DeltaMut::HaulLoadFactor((_, v)) => **v = 0.,
            DeltaMut::PropellorDiameter((_, v)) => **v = 0.,
            DeltaMut::PrismaticCoefficient((_, v)) => **v = 0.,
            DeltaMut::BlockCoefficent((_, v)) => **v = 0.,
            DeltaMut::PropellorEfficency((_, v)) => **v = 0.,
            DeltaMut::ShaftEfficiency((_, v)) => **v = 0.,
            DeltaMut::MidshipSectionCoefficient((_, v)) => **v = 0.,
            DeltaMut::SternParameter((_, v)) => **v = 0.,
        }
    }

    pub fn value(&self) -> Delta {
        match self {
            DeltaMut::HaulLoadFactor((g, v)) => Delta::HaulLoadFactor((*g, **v)),
            DeltaMut::PropellorDiameter((id, v)) => Delta::PropellorDiameter((*id, **v)),
            DeltaMut::PrismaticCoefficient((id, v)) => Delta::PrismaticCoefficient((*id, **v)),
            DeltaMut::BlockCoefficent((id, v)) => Delta::BlockCoefficent((*id, **v)),
            DeltaMut::PropellorEfficency((id, v)) => Delta::PropellorEfficency((*id, **v)),
            DeltaMut::ShaftEfficiency((id, v)) => Delta::ShaftEfficiency((*id, **v)),
            DeltaMut::MidshipSectionCoefficient((id, v)) => {
                Delta::MidshipSectionCoefficient((*id, **v))
            }
            DeltaMut::SternParameter((id, v)) => Delta::SternParameter((*id, **v)),
        }
    }

    pub fn neg(&mut self) {
        match self {
            DeltaMut::HaulLoadFactor((_, v)) => **v = -**v,
            DeltaMut::PropellorDiameter((_, v)) => **v = -**v,
            DeltaMut::PrismaticCoefficient((_, v)) => **v = -**v,
            DeltaMut::BlockCoefficent((_, v)) => **v = -**v,
            DeltaMut::PropellorEfficency((_, v)) => **v = -**v,
            DeltaMut::ShaftEfficiency((_, v)) => **v = -**v,
            DeltaMut::MidshipSectionCoefficient((_, v)) => **v = -**v,
            DeltaMut::SternParameter((_, v)) => **v = -**v,
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
            Delta::HaulLoadFactor((gear, delta)) => {
                self.haul_load_factors
                    .entry(gear)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing gear: {gear}"));
            }
            Delta::PropellorDiameter((id, delta)) => {
                self.propellor_diameter
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::PrismaticCoefficient((id, delta)) => {
                self.prismatic_coefficient
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::BlockCoefficent((id, delta)) => {
                self.block_coefficent
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::PropellorEfficency((id, delta)) => {
                self.propellor_efficency
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::ShaftEfficiency((id, delta)) => {
                self.shaft_efficiency
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::MidshipSectionCoefficient((id, delta)) => {
                self.midship_section_coefficient
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
            }
            Delta::SternParameter((id, delta)) => {
                self.stern_parameter
                    .entry(id)
                    .and_modify(|v| *v = (*v + delta).max(0.))
                    .or_insert_with(|| panic!("missing vessel id: {id}"));
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
            Delta::HaulLoadFactor((_, v)) => *v *= rhs,
            Delta::PropellorDiameter((_, v)) => *v *= rhs,
            Delta::PrismaticCoefficient((_, v)) => *v *= rhs,
            Delta::BlockCoefficent((_, v)) => *v *= rhs,
            Delta::PropellorEfficency((_, v)) => *v *= rhs,
            Delta::ShaftEfficiency((_, v)) => *v *= rhs,
            Delta::MidshipSectionCoefficient((_, v)) => *v *= rhs,
            Delta::SternParameter((_, v)) => *v *= rhs,
        };
        self
    }
}
