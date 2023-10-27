#[derive(Debug, Clone)]
pub enum MeilisearchError {
    Insert,
    Delete,
    Query,
    Index,
    Source,
    DataConversion,
}

impl std::error::Error for MeilisearchError {}

impl std::fmt::Display for MeilisearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeilisearchError::Insert => f.write_str("an error occured while inserting documents"),
            MeilisearchError::Delete => f.write_str("an error occured while deleting documents"),
            MeilisearchError::Query => f.write_str("an error occured while querying documents"),
            MeilisearchError::Index => f.write_str("an error occured while managing an index"),
            MeilisearchError::Source => {
                f.write_str("an error occured while fetching data from source")
            }
            MeilisearchError::DataConversion => f.write_str("failed to convert data"),
        }
    }
}
