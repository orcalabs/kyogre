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
