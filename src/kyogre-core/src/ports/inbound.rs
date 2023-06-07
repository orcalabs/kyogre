use std::collections::HashSet;

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;
use fiskeridir_rs::LandingId;

#[async_trait]
pub trait AisMigratorDestination {
    async fn migrate_ais_data(
        &self,
        mmsi: Mmsi,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), InsertError>;
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, QueryError>;
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
    async fn add_fishing_facilities(
        &self,
        facilities: Vec<FishingFacility>,
    ) -> Result<(), InsertError>;
    async fn add_register_vessels(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> Result<(), InsertError>;
    async fn add_landings(
        &self,
        landings: Vec<fiskeridir_rs::Landing>,
        data_year: u32,
    ) -> Result<(), InsertError>;
    async fn delete_removed_landings(
        &self,
        existing_landing_ids: HashSet<LandingId>,
        data_year: u32,
    ) -> Result<(), DeleteError>;
    async fn delete_ers_dca(&self, year: u32) -> Result<(), DeleteError>;
    async fn add_ers_dca(&self, ers_dca: Vec<fiskeridir_rs::ErsDca>) -> Result<(), InsertError>;
    async fn add_ers_dep(&self, ers_dep: Vec<fiskeridir_rs::ErsDep>) -> Result<(), InsertError>;
    async fn delete_ers_dep(&self, year: u32) -> Result<(), DeleteError>;
    async fn add_ers_por(&self, ers_por: Vec<fiskeridir_rs::ErsPor>) -> Result<(), InsertError>;
    async fn delete_ers_por(&self, year: u32) -> Result<(), DeleteError>;
    async fn add_ers_tra(&self, ers_tra: Vec<fiskeridir_rs::ErsTra>) -> Result<(), InsertError>;
    async fn delete_ers_tra_catches(&self, year: u32) -> Result<(), DeleteError>;
    async fn add_vms(&self, vms: Vec<fiskeridir_rs::Vms>) -> Result<(), InsertError>;
}

#[async_trait]
pub trait ScraperOutboundPort {
    async fn latest_fishing_facility_update(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> Result<Option<DateTime<Utc>>, QueryError>;
}

#[async_trait]
pub trait ScraperFileHashInboundPort {
    async fn add(&self, id: &FileHashId, hash: String) -> Result<(), InsertError>;
    async fn diff(&self, id: &FileHashId, hash: &str) -> Result<HashDiff, QueryError>;
}

#[async_trait]
pub trait VesselBenchmarkInbound: Send + Sync {
    async fn add_output(&self, values: Vec<VesselBenchmarkOutput>) -> Result<(), InsertError>;
}

#[async_trait]
pub trait HaulDistributorInbound: Send + Sync {
    async fn add_output(&self, values: Vec<HaulDistributionOutput>) -> Result<(), UpdateError>;
}
