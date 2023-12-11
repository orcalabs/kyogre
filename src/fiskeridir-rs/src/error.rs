use error_stack::Context;

#[derive(Debug)]
pub enum Error {
    Download,
    Deserialize,
    Hash,
    Conversion,
    IncompleteData,
}

impl Context for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Download => f.write_str("error downloading data source"),
            Error::Deserialize => f.write_str("error deserializing data source"),
            Error::Hash => f.write_str("error hashing data file"),
            Error::Conversion => {
                f.write_str("error converting between message model and internal model")
            }
            Error::IncompleteData => {
                f.write_str("encountered incomplete data, one or more columns are missing")
            }
        }
    }
}
