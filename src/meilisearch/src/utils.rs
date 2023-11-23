use std::{fmt::Display, ops::Bound};

use chrono::{DateTime, Utc};
use error_stack::{report, Result};
use kyogre_core::Range;

use crate::error::MeilisearchError;

pub fn to_nanos(timestamp: DateTime<Utc>) -> Result<i64, MeilisearchError> {
    timestamp.timestamp_nanos_opt().ok_or_else(|| {
        report!(MeilisearchError::DataConversion).attach_printable(format!(
            "{} could not be converted to timestamp nanos",
            timestamp
        ))
    })
}

pub fn join_comma<T: ToString>(values: Vec<T>) -> String {
    values
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
pub fn join_comma_fn<T, S: ToString, F: Fn(T) -> S>(values: Vec<T>, closure: F) -> String {
    values
        .into_iter()
        .map(|v| closure(v).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn create_ranges_filter<I, T, S>(ranges: I, attr1: S, attr2: S) -> String
where
    I: IntoIterator<Item = Range<T>>,
    T: Display,
    S: Display,
{
    ranges
        .into_iter()
        .flat_map(|range| {
            let start = match range.start {
                Bound::Included(v) => Some(format!("{attr1} >= {v}")),
                Bound::Excluded(v) => Some(format!("{attr1} > {v}")),
                Bound::Unbounded => None,
            };
            let end = match range.end {
                Bound::Included(v) => Some(format!("{attr2} <= {v}")),
                Bound::Excluded(v) => Some(format!("{attr2} < {v}")),
                Bound::Unbounded => None,
            };
            match (start, end) {
                (Some(a), Some(b)) => Some(format!("({a} AND {b})")),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}
