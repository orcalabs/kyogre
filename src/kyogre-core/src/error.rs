use chrono::{DateTime, Utc};
use error_stack::Context;

#[derive(Debug)]
pub struct InsertError;

impl Context for InsertError {}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred during data insertion")
    }
}

#[derive(Debug)]
pub struct QueryError;

impl Context for QueryError {}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred during data retrieval")
    }
}

#[derive(Debug)]
pub struct UpdateError;

impl Context for UpdateError {}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred while updating data")
    }
}

#[derive(Debug)]
pub struct DeleteError;

impl Context for DeleteError {}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred while deleting data")
    }
}

#[derive(Debug)]
pub enum DateRangeError {
    Ordering(DateTime<Utc>, DateTime<Utc>),
    Unbounded,
}

impl std::error::Error for DateRangeError {}

impl std::fmt::Display for DateRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateRangeError::Ordering(start, end) => f.write_fmt(format_args!(
                "start of date range cannot be after end, start: {}, end: {}",
                start, end
            )),
            DateRangeError::Unbounded => f.write_str("encountered and unexpected unbounded range"),
        }
    }
}

#[derive(Debug)]
pub struct ConversionError;

impl Context for ConversionError {}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred during data conversion")
    }
}

#[derive(Debug)]
pub enum BearerTokenError {
    Configuration,
    Acquisition,
}

impl Context for BearerTokenError {}

impl std::fmt::Display for BearerTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BearerTokenError::Configuration => f.write_str("invalid oauth configuration"),
            BearerTokenError::Acquisition => f.write_str("failed to acquire token"),
        }
    }
}

#[derive(Debug)]
pub enum HaulMatrixIndexError {
    Date(i32),
    GearGroup(i32),
    VesselLength(i32),
    SpeciesGroup(i32),
}

impl std::error::Error for HaulMatrixIndexError {}

impl std::fmt::Display for HaulMatrixIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaulMatrixIndexError::Date(v) => f.write_fmt(format_args!(
                "encountered a month bucket older than the oldest existing ers data: {}",
                v
            )),
            HaulMatrixIndexError::GearGroup(v) => {
                f.write_fmt(format_args!("encountered an unknown gear group: {}", v))
            }
            HaulMatrixIndexError::VesselLength(v) => f.write_fmt(format_args!(
                "encountered an unknown vessel length group: {}",
                v
            )),
            HaulMatrixIndexError::SpeciesGroup(v) => {
                f.write_fmt(format_args!("encountered an species group {}", v))
            }
        }
    }
}
