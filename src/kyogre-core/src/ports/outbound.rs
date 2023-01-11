use crate::{AisPosition, QueryError};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;

#[async_trait]
pub trait AisMigratorSource {
    async fn ais_positions(
        &self,
        mmsi: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AisPosition>, QueryError>;
    async fn existing_mmsis(&self) -> Result<Vec<i32>, QueryError>;
}
