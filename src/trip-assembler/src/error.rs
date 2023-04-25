use error_stack::Context;
use geoutils::Location;

#[derive(Debug)]
pub struct TripAssemblerError;

impl Context for TripAssemblerError {}

impl std::fmt::Display for TripAssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured during trip assembly")
    }
}

#[derive(Debug)]
pub struct TripPrecisionError;

impl Context for TripPrecisionError {}

impl std::fmt::Display for TripPrecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured during trip precision calculation")
    }
}

#[derive(Debug)]
pub struct LocationDistanceToError {
    pub from: Location,
    pub to: Location,
}

impl std::error::Error for LocationDistanceToError {}

impl std::fmt::Display for LocationDistanceToError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "an error occured when calculating distance from {:?} to {:?}",
            self.from, self.to
        ))
    }
}
