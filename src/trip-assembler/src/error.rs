use error_stack::Context;

#[derive(Debug)]
pub struct TripAssemblerError;

impl Context for TripAssemblerError {}

impl std::fmt::Display for TripAssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured during trip assembly")
    }
}
