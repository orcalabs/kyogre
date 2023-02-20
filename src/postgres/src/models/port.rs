use std::str::FromStr;

use error_stack::{report, Result};
use jurisdiction::Jurisdiction;

use crate::error::PostgresError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPort {
    pub id: String,
    pub name: Option<String>,
    pub nationality: String,
}

impl NewPort {
    pub fn new(id: String, name: Option<String>) -> Result<Self, PostgresError> {
        let jurisdiction = Jurisdiction::from_str(&id[0..2])
            .map_err(|e| report!(PostgresError::DataConversion).attach_printable(format!("{e}")))?;

        Ok(Self {
            id,
            name,
            nationality: jurisdiction.alpha3().to_string(),
        })
    }
}
