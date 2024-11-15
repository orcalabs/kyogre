use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::{DataFileId, SpeciesGroup};

pub type BoxIterator<T> = Box<dyn Iterator<Item = T> + Send + Sync>;

#[async_trait]
pub trait MLModelsInbound: Send + Sync {
    async fn add_fishing_spot_predictions(
        &self,
        predictions: Vec<NewFishingSpotPrediction>,
    ) -> CoreResult<()>;
    async fn add_fishing_weight_predictions(
        &self,
        predictions: Vec<NewFishingWeightPrediction>,
    ) -> CoreResult<()>;
    async fn catch_locations(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> CoreResult<Vec<CatchLocation>>;
    async fn existing_fishing_spot_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> CoreResult<Vec<FishingSpotPrediction>>;
    async fn existing_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> CoreResult<Vec<FishingWeightPrediction>>;
    async fn catch_locations_weather_dates(
        &self,
        dates: Vec<NaiveDate>,
    ) -> CoreResult<Vec<CatchLocationWeather>>;
    async fn catch_locations_weather(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> CoreResult<Vec<CatchLocationWeather>>;
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
    ) -> CoreResult<()>;
    async fn add_mmsis(&self, mmsi: &[Mmsi]) -> CoreResult<()>;
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> CoreResult<Vec<AisVesselMigrate>>;
}

#[async_trait]
pub trait WebApiInboundPort {
    async fn update_user(&self, user: &User) -> CoreResult<()>;
    async fn add_fuel_measurements(&self, measurements: &[FuelMeasurement]) -> CoreResult<()>;
    async fn update_fuel_measurements(&self, measurements: &[FuelMeasurement]) -> CoreResult<()>;
    async fn delete_fuel_measurements(
        &self,
        measurements: &[DeleteFuelMeasurement],
    ) -> CoreResult<()>;
}

#[async_trait]
pub trait ScraperInboundPort {
    async fn add_fishing_facilities(&self, facilities: Vec<FishingFacility>) -> CoreResult<()>;
    async fn add_register_vessels(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> CoreResult<()>;
    async fn add_landings(
        &self,
        landings: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::Landing>>,
        data_year: u32,
    ) -> CoreResult<()>;
    async fn add_ers_dca(
        &self,
        ers_dca: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsDca>>,
    ) -> CoreResult<()>;
    async fn add_ers_dep(
        &self,
        ers_dep: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsDep>>,
    ) -> CoreResult<()>;
    async fn add_ers_por(
        &self,
        ers_por: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsPor>>,
    ) -> CoreResult<()>;
    async fn add_ers_tra(
        &self,
        ers_tra: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsTra>>,
    ) -> CoreResult<()>;
    async fn add_vms(&self, vms: Vec<fiskeridir_rs::Vms>) -> CoreResult<()>;
    async fn add_aqua_culture_register(
        &self,
        entries: Vec<fiskeridir_rs::AquaCultureEntry>,
    ) -> CoreResult<()>;
    async fn add_mattilsynet_delivery_points(
        &self,
        delivery_points: Vec<MattilsynetDeliveryPoint>,
    ) -> CoreResult<()>;
    async fn add_weather(&self, weather: Vec<NewWeather>) -> CoreResult<()>;
    async fn add_ocean_climate(&self, ocean_climate: Vec<NewOceanClimate>) -> CoreResult<()>;
}

#[async_trait]
pub trait ScraperOutboundPort {
    async fn latest_fishing_facility_update(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> CoreResult<Option<DateTime<Utc>>>;
    async fn latest_weather_timestamp(&self) -> CoreResult<Option<DateTime<Utc>>>;
    async fn latest_ocean_climate_timestamp(&self) -> CoreResult<Option<DateTime<Utc>>>;
}

#[async_trait]
pub trait ScraperFileHashInboundPort {
    async fn add(&self, id: &DataFileId, hash: String) -> CoreResult<()>;
}

#[async_trait]
pub trait TripBenchmarkInbound: Send + Sync {
    async fn add_output(&self, values: Vec<TripBenchmarkOutput>) -> CoreResult<()>;
    async fn refresh_trips(&self) -> CoreResult<()>;
}

#[async_trait]
pub trait HaulDistributorInbound: Send + Sync {
    async fn add_output(&self, values: Vec<HaulDistributionOutput>) -> CoreResult<()>;
    async fn update_bycatch_status(&self) -> CoreResult<()>;
}

#[async_trait]
pub trait TripPipelineInbound: Send + Sync {
    async fn reset_trip_processing_conflicts(&self) -> CoreResult<()>;
    async fn update_preferred_trip_assemblers(&self) -> CoreResult<()>;
    async fn update_trip(&self, update: TripUpdate) -> CoreResult<()>;
    async fn add_trip_set(&self, value: TripSet) -> CoreResult<()>;
    async fn refresh_detailed_trips(&self, vessel_id: FiskeridirVesselId) -> CoreResult<()>;
}

#[cfg(feature = "test")]
#[async_trait]
pub trait TestHelperInbound: Send + Sync {
    async fn manual_vessel_conflict_override(&self, conflicts: Vec<NewVesselConflict>);
    async fn queue_trip_reset(&self);
    async fn clear_trip_distancing(&self, vessel_id: FiskeridirVesselId);
    async fn clear_trip_precision(&self, vessel_id: FiskeridirVesselId);
    async fn add_manual_delivery_points(&self, values: Vec<ManualDeliveryPoint>);
    async fn add_deprecated_delivery_point(
        &self,
        old: fiskeridir_rs::DeliveryPointId,
        new: fiskeridir_rs::DeliveryPointId,
    ) -> CoreResult<()>;
}

#[async_trait]
pub trait HaulWeatherInbound: Send + Sync {
    async fn add_haul_weather(&self, values: Vec<HaulWeatherOutput>) -> CoreResult<()>;
}

#[async_trait]
pub trait DailyWeatherInbound: Send + Sync {
    async fn catch_locations_with_weather(&self) -> CoreResult<Vec<CatchLocationId>>;
    async fn dirty_dates(&self) -> CoreResult<Vec<NaiveDate>>;
    async fn prune_dirty_dates(&self) -> CoreResult<()>;
    async fn update_daily_weather(
        &self,
        catch_locations: &[CatchLocationId],
        date: NaiveDate,
    ) -> CoreResult<()>;
}

#[async_trait]
pub trait AisVmsAreaPrunerInbound: Send + Sync {
    async fn prune_ais_vms_area(&self, limit: NaiveDate) -> CoreResult<()>;
}
