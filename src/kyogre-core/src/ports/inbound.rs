use crate::{
    AisPosition, AisVesselMigrate, FileHashId, HashDiff, InsertError, NewTrip, QueryError,
    TripPrecisionUpdate, TripsConflictStrategy, UpdateError,
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

#[async_trait]
pub trait TripPrecisionInboundPort {
    async fn update_trip_precisions(
        &self,
        updates: Vec<TripPrecisionUpdate>,
    ) -> Result<(), UpdateError>;
}

#[async_trait]
pub trait ScraperInboundPort {
    async fn add_landings(&self, landings: Vec<fiskeridir_rs::Landing>) -> Result<(), InsertError>;
    async fn delete_ers_dca(&self) -> Result<(), InsertError>;
    async fn add_ers_dca(&self, ers_dca: Vec<fiskeridir_rs::ErsDca>) -> Result<(), InsertError>;
    async fn add_ers_dep(&self, ers_dep: Vec<fiskeridir_rs::ErsDep>) -> Result<(), InsertError>;
    async fn add_ers_por(&self, ers_por: Vec<fiskeridir_rs::ErsPor>) -> Result<(), InsertError>;
    async fn update_database_views(&self) -> Result<(), UpdateError>;
}

#[async_trait]
pub trait ScraperFileHashInboundPort {
    async fn add(&self, id: &FileHashId, hash: String) -> Result<(), InsertError>;
    async fn diff(&self, id: &FileHashId, hash: &str) -> Result<HashDiff, QueryError>;
}
