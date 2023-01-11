use error_stack::Context;

#[derive(Debug)]
pub enum MigratorError {
    Source,
    Destination,
}

impl Context for MigratorError {}

impl std::fmt::Display for MigratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigratorError::Source => f.write_str("an error occurred at the ais source"),
            MigratorError::Destination => f.write_str("an error occurred at the ais destination"),
        }
    }
}
