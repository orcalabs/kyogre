use bigdecimal::BigDecimal;
use error_stack::Context;

#[derive(Debug)]
pub enum PostgresError {
    Connection,
    Transaction,
    Query,
    DataConversion,
}

impl Context for PostgresError {}

impl std::fmt::Display for PostgresError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostgresError::Connection => f.write_str("failed to acquire a database connection"),
            PostgresError::Transaction => f.write_str("failed to start/commit transaction"),
            PostgresError::Query => f.write_str("a query related error occured"),
            PostgresError::DataConversion => {
                f.write_str("failed to convert data to postgres specific data type")
            }
        }
    }
}

#[derive(Debug)]
pub struct UnboundedRangeError;

impl std::error::Error for UnboundedRangeError {}

impl std::fmt::Display for UnboundedRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("encountered and unexpected unbounded range")
    }
}

#[derive(Debug)]
pub struct BigDecimalError(pub f64);

impl std::error::Error for BigDecimalError {}

impl std::fmt::Display for BigDecimalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "failed to convert float value to big decimal, value: {}",
            self.0
        ))
    }
}

#[derive(Debug)]
pub struct FromBigDecimalError(pub BigDecimal);

impl std::error::Error for FromBigDecimalError {}

impl std::fmt::Display for FromBigDecimalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "failed to convert bigdecimal value to float, value: {}",
            self.0
        ))
    }
}

#[derive(Debug)]
pub struct NavigationStatusError(pub i32);

impl std::error::Error for NavigationStatusError {}

impl std::fmt::Display for NavigationStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "failed to convert int value to navigation status, value: {}",
            self.0
        ))
    }
}
