use std::{
    cmp::Ordering,
    ops::{Add, AddAssign, Mul, Neg, SubAssign},
};

use crate::Trip;

pub mod holtrop;
pub mod maru;

#[derive(Debug, Clone)]
pub enum ParamVariants {
    Maru(maru::Params),
    Holtrop(holtrop::Params),
}

#[derive(Debug, Clone)]
pub enum ParamDeltaVariants {
    Maru(maru::ParamsDelta),
    Holtrop(holtrop::ParamsDelta),
}

#[derive(Debug, Clone, Copy)]
pub enum DeltaVariants {
    Maru(maru::Delta),
    Holtrop(holtrop::Delta),
}

#[derive(Debug)]
pub enum DeltaMutVariants<'a> {
    Maru(maru::DeltaMut<'a>),
    Holtrop(holtrop::DeltaMut<'a>),
}

impl ParamDeltaVariants {
    pub fn iter_mut(&mut self) -> Box<dyn Iterator<Item = DeltaMutVariants<'_>> + '_> {
        match self {
            ParamDeltaVariants::Maru(params_delta) => {
                Box::new(params_delta.iter_mut().map(DeltaMutVariants::Maru))
            }
            ParamDeltaVariants::Holtrop(params_delta) => {
                Box::new(params_delta.iter_mut().map(DeltaMutVariants::Holtrop))
            }
        }
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = DeltaVariants> + '_> {
        match self {
            ParamDeltaVariants::Maru(params_delta) => {
                Box::new(params_delta.iter().map(DeltaVariants::Maru))
            }
            ParamDeltaVariants::Holtrop(params_delta) => {
                Box::new(params_delta.iter().map(DeltaVariants::Holtrop))
            }
        }
    }
}

impl DeltaMutVariants<'_> {
    pub fn is_zero(&self) -> bool {
        match self {
            DeltaMutVariants::Maru(delta_mut) => delta_mut.is_zero(),
            DeltaMutVariants::Holtrop(delta_mut) => delta_mut.is_zero(),
        }
    }

    pub fn set_zero(&mut self) {
        match self {
            DeltaMutVariants::Maru(delta_mut) => delta_mut.set_zero(),
            DeltaMutVariants::Holtrop(delta_mut) => delta_mut.set_zero(),
        }
    }

    pub fn value(&self) -> DeltaVariants {
        match self {
            DeltaMutVariants::Maru(delta_mut) => DeltaVariants::Maru(delta_mut.value()),
            DeltaMutVariants::Holtrop(delta_mut) => DeltaVariants::Holtrop(delta_mut.value()),
        }
    }

    pub fn neg(&mut self) {
        match self {
            DeltaMutVariants::Maru(delta_mut) => delta_mut.neg(),
            DeltaMutVariants::Holtrop(delta_mut) => delta_mut.neg(),
        }
    }
}

impl AddAssign<&ParamDeltaVariants> for ParamVariants {
    fn add_assign(&mut self, rhs: &ParamDeltaVariants) {
        for d in rhs.iter() {
            *self += d;
        }
    }
}

impl Add<&ParamDeltaVariants> for ParamVariants {
    type Output = Self;

    fn add(mut self, rhs: &ParamDeltaVariants) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<DeltaVariants> for ParamVariants {
    fn add_assign(&mut self, rhs: DeltaVariants) {
        match self {
            ParamVariants::Maru(params) => {
                let DeltaVariants::Maru(val) = rhs else {
                    unimplemented!();
                };
                params.add_assign(val)
            }
            ParamVariants::Holtrop(params) => {
                let DeltaVariants::Holtrop(val) = rhs else {
                    unimplemented!();
                };
                params.add_assign(val)
            }
        }
    }
}

impl Neg for DeltaVariants {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            DeltaVariants::Maru(delta) => {
                let new = delta.neg();
                DeltaVariants::Maru(new)
            }
            DeltaVariants::Holtrop(delta) => {
                let new = delta.neg();
                DeltaVariants::Holtrop(new)
            }
        }
    }
}

impl SubAssign<DeltaVariants> for ParamVariants {
    fn sub_assign(&mut self, rhs: DeltaVariants) {
        match self {
            ParamVariants::Maru(params) => {
                let DeltaVariants::Maru(val) = rhs else {
                    unimplemented!();
                };
                params.sub_assign(val)
            }
            ParamVariants::Holtrop(params) => {
                let DeltaVariants::Holtrop(val) = rhs else {
                    unimplemented!();
                };
                params.sub_assign(val)
            }
        }
    }
}

impl Mul<f64> for DeltaVariants {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        match self {
            DeltaVariants::Maru(delta) => {
                let new = delta.mul(rhs);
                DeltaVariants::Maru(new)
            }
            DeltaVariants::Holtrop(delta) => {
                let new = delta.mul(rhs);
                DeltaVariants::Holtrop(new)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    pub mean: f64,
    pub sd: f64,
}

impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score().total_cmp(&other.score())
    }
}

impl Score {
    pub fn new(trips: &[Trip], params: &ParamVariants) -> Self {
        let diffs = trips
            .iter()
            .map(|v| v.diff_percent(params).abs())
            .collect::<Vec<_>>();

        let n = diffs.len() as f64;
        let mean = diffs.iter().sum::<f64>() / n;
        let sd = (diffs
            .iter()
            .map(|v| ((v - mean).abs().powf(2.)))
            .sum::<f64>()
            / n)
            .sqrt();

        assert!(mean >= 0., "mean: {mean}");
        assert!(sd >= 0., "sd: {sd}");

        Self { mean, sd }
    }

    #[inline]
    fn score(&self) -> f64 {
        self.mean + self.sd
    }
}
