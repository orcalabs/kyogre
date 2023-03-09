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
pub struct DateRangeError {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl std::error::Error for DateRangeError {}

impl std::fmt::Display for DateRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "start of date range cannot be after end, start: {}, end: {}",
            self.start, self.end
        ))
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
