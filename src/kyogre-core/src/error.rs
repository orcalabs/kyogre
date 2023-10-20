use std::fmt::Display;

use chrono::{DateTime, Utc};
use error_stack::Context;

#[derive(Debug)]
pub struct InsertError;

impl Context for InsertError {}

impl Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred during data insertion")
    }
}

#[derive(Debug)]
pub struct QueryError;

impl Context for QueryError {}

impl Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred during data retrieval")
    }
}

#[derive(Debug)]
pub struct UpdateError;

impl Context for UpdateError {}

impl Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred while updating data")
    }
}

#[derive(Debug)]
pub struct DeleteError;

impl Context for DeleteError {}

impl Display for DeleteError {
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

impl Display for DateRangeError {
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

impl Display for ConversionError {
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

impl Display for BearerTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BearerTokenError::Configuration => f.write_str("invalid oauth configuration"),
            BearerTokenError::Acquisition => f.write_str("failed to acquire token"),
        }
    }
}
#[derive(Debug)]
pub enum LandingMatrixIndexError {
    Date(i32),
    GearGroup(i32),
    VesselLength(i32),
    SpeciesGroup(i32),
}

impl std::error::Error for LandingMatrixIndexError {}

impl Display for LandingMatrixIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LandingMatrixIndexError::Date(v) => f.write_fmt(format_args!(
                "encountered a month bucket older than the oldest existing landing data: {}",
                v
            )),
            LandingMatrixIndexError::GearGroup(v) => {
                f.write_fmt(format_args!("encountered an unknown gear group: {}", v))
            }
            LandingMatrixIndexError::VesselLength(v) => f.write_fmt(format_args!(
                "encountered an unknown vessel length group: {}",
                v
            )),
            LandingMatrixIndexError::SpeciesGroup(v) => {
                f.write_fmt(format_args!("encountered an species group {}", v))
            }
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

impl Display for HaulMatrixIndexError {
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

#[derive(Debug)]
pub struct TripAssemblerError;

impl Context for TripAssemblerError {}

impl Display for TripAssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured during trip assembly")
    }
}

#[derive(Debug)]
pub struct TripPrecisionError;

impl Context for TripPrecisionError {}

impl Display for TripPrecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured during trip precision calculation")
    }
}

#[derive(Debug)]
pub struct HaulWeatherError;

impl std::error::Error for HaulWeatherError {}

impl Display for HaulWeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured during haul weather processing")
    }
}

#[derive(Debug)]
pub enum EngineError {
    Transition,
}

impl Context for EngineError {}

impl Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Transition => {
                f.write_str("an error occured when trying to figure out the next state transition")
            }
        }
    }
}

#[derive(Debug)]
pub struct BenchmarkError;

impl Context for BenchmarkError {}

impl Display for BenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a benchmark")
    }
}

#[derive(Debug)]
pub struct HaulDistributorError;

impl std::error::Error for HaulDistributorError {}

impl Display for HaulDistributorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a haul distributor")
    }
}

#[derive(Debug)]
pub enum TripPipelineError {
    NewTripProcessing,
    ExistingTripProcessing,
    DataPreparation,
}

impl Display for TripPipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TripPipelineError::NewTripProcessing => {
                f.write_str("an error occured while processing new trips")
            }
            TripPipelineError::ExistingTripProcessing => {
                f.write_str("an error occured while processing existing trips")
            }
            TripPipelineError::DataPreparation => {
                f.write_str("an error occured while preparing data for processing")
            }
        }
    }
}

impl Context for TripPipelineError {}

#[derive(Debug)]
pub struct TripLayerError;

impl Context for TripLayerError {}

impl Display for TripLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a trip layer")
    }
}
