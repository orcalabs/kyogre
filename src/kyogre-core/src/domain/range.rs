use std::fmt::Debug;
use std::fmt::Display;
use std::ops::Bound;
use std::str::FromStr;

use error_stack::{report, Report};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Range<T> {
    pub start: Bound<T>,
    pub end: Bound<T>,
}

#[derive(Debug)]
pub enum RangeError {
    InvalidLength,
    InvalidLowerBound,
    InvalidStartValue,
    InvalidUpperBound,
    InvalidEndValue,
}

impl std::error::Error for RangeError {}

impl std::fmt::Display for RangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RangeError::InvalidLength => f.write_str("range did not contain a valid length"),
            RangeError::InvalidLowerBound => {
                f.write_str("range did not contain a valid lower bound")
            }
            RangeError::InvalidStartValue => {
                f.write_str("range did not contain a valid start value")
            }
            RangeError::InvalidUpperBound => {
                f.write_str("range did not contain a valid upper bound")
            }
            RangeError::InvalidEndValue => f.write_str("range did not contain a valid end value"),
        }
    }
}

impl<T: FromStr> FromStr for Range<T> {
    type Err = Report<RangeError>;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let split = v.split(',').collect::<Vec<_>>();

        if split.len() != 2 {
            return Err(report!(RangeError::InvalidLength));
        }
        if split[0].is_empty() {
            return Err(report!(RangeError::InvalidLowerBound));
        }
        if split[1].is_empty() {
            return Err(report!(RangeError::InvalidUpperBound));
        }

        let (lower_bound, start_str) = split[0].split_at(1);
        let start = match start_str.len() {
            0 => Ok(Bound::Unbounded),
            _ => {
                let start_value = start_str
                    .parse::<T>()
                    .map_err(|_| report!(RangeError::InvalidStartValue))?;

                match lower_bound {
                    "(" => Ok(Bound::Excluded(start_value)),
                    "[" => Ok(Bound::Included(start_value)),
                    _ => Err(report!(RangeError::InvalidLowerBound)),
                }
            }
        }?;

        let (end_str, upper_bound) = split[1].split_at(split[1].len() - 1);
        let end = match end_str.len() {
            0 => Ok(Bound::Unbounded),
            _ => {
                let end_value = end_str
                    .parse::<T>()
                    .map_err(|_| report!(RangeError::InvalidEndValue))?;

                match upper_bound {
                    ")" => Ok(Bound::Excluded(end_value)),
                    "]" => Ok(Bound::Included(end_value)),
                    _ => Err(report!(RangeError::InvalidUpperBound)),
                }
            }
        }?;

        Ok(Range { start, end })
    }
}

impl<T: FromStr> TryFrom<String> for Range<T> {
    type Error = Report<RangeError>;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        Self::from_str(v.as_str())
    }
}

impl<T: ToString + Display> ToString for Range<T> {
    fn to_string(&self) -> String {
        format!(
            "{},{}",
            match &self.start {
                Bound::Unbounded => "(".into(),
                Bound::Excluded(v) => format!("({v}"),
                Bound::Included(v) => format!("[{v}"),
            },
            match &self.end {
                Bound::Unbounded => ")".into(),
                Bound::Excluded(v) => format!("{v})"),
                Bound::Included(v) => format!("{v}]"),
            },
        )
    }
}