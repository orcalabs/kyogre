use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;
use fiskeridir_rs::{DeliveryPointId, SpeciesGroup};

#[async_trait]
pub trait MLModelsInbound: Send + Sync {
    async fn add_fishing_spot_predictions(
        &self,
        predictions: Vec<NewFishingSpotPrediction>,
    ) -> Result<(), InsertError>;
    async fn add_fishing_weight_predictions(
        &self,
        predictions: Vec<NewFishingWeightPrediction>,
    ) -> Result<(), InsertError>;
    async fn catch_locations(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> Result<Vec<CatchLocation>, QueryError>;
    async fn existing_fishing_spot_predictions(
        &self,
        model_id: ModelId,
        year: u32,
    ) -> Result<Vec<FishingSpotPrediction>, QueryError>;
    async fn existing_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        year: u32,
    ) -> Result<Vec<FishingWeightPrediction>, QueryError>;
    async fn species_caught_with_traal(&self) -> Result<Vec<SpeciesGroup>, QueryError>;
    async fn catch_location_weather(
        &self,
        year: u32,
        week: u32,
        catch_location_id: &CatchLocationId,
    ) -> Result<Option<CatchLocationWeather>, QueryError>;
}

#[async_trait]
pub trait AisConsumeLoop: Sync + Send {
    async fn consume(
        &self,
        mut receiver: tokio::sync::broadcast::Receiver<DataMessage>,
        process_confirmation: Option<tokio::sync::mpsc::Sender<()>>,
    );
}

#[async_trait]
pub trait AisMigratorDestination {
    async fn migrate_ais_data(
        &self,
        mmsi: Mmsi,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), InsertError>;
    async fn add_mmsis(&self, mmsi: Vec<Mmsi>) -> Result<(), InsertError>;
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, QueryError>;
}

#[async_trait]
pub trait WebApiInboundPort {
    async fn update_user(&self, user: User) -> Result<(), UpdateError>;
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
        landings: Box<
            dyn Iterator<Item = Result<fiskeridir_rs::Landing, fiskeridir_rs::Error>> + Send + Sync,
        >,
        data_year: u32,
    ) -> Result<(), InsertError>;
    async fn add_ers_dca(
        &self,
        ers_dca: Box<
            dyn Iterator<Item = Result<fiskeridir_rs::ErsDca, fiskeridir_rs::Error>> + Send + Sync,
        >,
    ) -> Result<(), InsertError>;
    async fn add_ers_dep(&self, ers_dep: Vec<fiskeridir_rs::ErsDep>) -> Result<(), InsertError>;
    async fn add_ers_por(&self, ers_por: Vec<fiskeridir_rs::ErsPor>) -> Result<(), InsertError>;
    async fn add_ers_tra(&self, ers_tra: Vec<fiskeridir_rs::ErsTra>) -> Result<(), InsertError>;
    async fn add_vms(&self, vms: Vec<fiskeridir_rs::Vms>) -> Result<(), InsertError>;
    async fn add_aqua_culture_register(
        &self,
        entries: Vec<fiskeridir_rs::AquaCultureEntry>,
    ) -> Result<(), InsertError>;
    async fn add_mattilsynet_delivery_points(
        &self,
        delivery_points: Vec<MattilsynetDeliveryPoint>,
    ) -> Result<(), InsertError>;
    async fn add_weather(&self, weather: Vec<NewWeather>) -> Result<(), InsertError>;
    async fn add_ocean_climate(
        &self,
        ocean_climate: Vec<NewOceanClimate>,
    ) -> Result<(), InsertError>;
}

#[async_trait]
pub trait ScraperOutboundPort {
    async fn latest_fishing_facility_update(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> Result<Option<DateTime<Utc>>, QueryError>;
    async fn latest_weather_timestamp(&self) -> Result<Option<DateTime<Utc>>, QueryError>;
    async fn latest_ocean_climate_timestamp(&self) -> Result<Option<DateTime<Utc>>, QueryError>;
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

#[async_trait]
pub trait TripPipelineInbound: Send + Sync {
    async fn update_preferred_trip_assemblers(&self) -> Result<(), UpdateError>;
    async fn update_trip(&self, update: TripUpdate) -> Result<(), UpdateError>;
    async fn add_trip_set(&self, value: TripSet) -> Result<(), InsertError>;
    async fn refresh_detailed_trips(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<(), UpdateError>;
}

#[async_trait]
pub trait TestHelperInbound: Send + Sync {
    async fn manual_vessel_conflict_override(&self, conflicts: Vec<NewVesselConflict>);
    async fn queue_trip_reset(&self);
    async fn clear_trip_distancing(&self, vessel_id: FiskeridirVesselId);
    async fn clear_trip_precision(&self, vessel_id: FiskeridirVesselId);
    async fn add_manual_delivery_points(&self, values: Vec<ManualDeliveryPoint>);
    async fn add_deprecated_delivery_point(
        &self,
        old: DeliveryPointId,
        new: DeliveryPointId,
    ) -> Result<(), InsertError>;
}

#[async_trait]
pub trait HaulWeatherInbound: Send + Sync {
    async fn add_haul_weather(&self, values: Vec<HaulWeatherOutput>) -> Result<(), UpdateError>;
}
