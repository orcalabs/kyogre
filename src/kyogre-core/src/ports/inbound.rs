use crate::{
    AisPosition, AisVesselMigrate, FileHashId, HashDiff, InsertError, NewTrip, QueryError,
    TripsConflictStrategy,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;

#[async_trait]
pub trait AisMigratorDestination {
    async fn migrate_ais_data(
        &self,
        mmsi: i32,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), InsertError>;
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, QueryError>;
}

#[async_trait]
pub trait TripAssemblerInboundPort {
    async fn add_trips(
        &self,
        vessel_id: i64,
        new_trip_calculation_time: DateTime<Utc>,
        conflict_strategy: TripsConflictStrategy,
        trips: Vec<NewTrip>,
    ) -> Result<Vec<DateTime<Utc>>, InsertError>;
}

pub trait ScraperInboundPort {}

#[async_trait]
pub trait ScraperFileHashInboundPort {
    async fn add(&self, id: &FileHashId, hash: String) -> Result<(), InsertError>;
    async fn diff(&self, id: &FileHashId, hash: &str) -> Result<HashDiff, QueryError>;
}
