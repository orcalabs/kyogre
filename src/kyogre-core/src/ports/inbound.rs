use crate::*;
use async_channel::Receiver;
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
        mut receiver: Receiver<DataMessage>,
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
    async fn add_weekly_sales(&self, weekly_sales: Vec<WeeklySale>) -> CoreResult<()>;
    async fn add_register_vessels(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> CoreResult<()>;
    async fn add_buyer_locations(&self, locations: Vec<BuyerLocation>) -> CoreResult<()>;
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
    /// Sets the current trip for the given vessel, only relevant for ERS-based assemblers
    /// and depends on the latest departure message.
    async fn set_current_trip(&self, vessel_id: FiskeridirVesselId) -> CoreResult<()>;
    /// Checks wether any vms data has been added out of order (e.g. vms data for '2024-04-04' was
    /// inserted at '2024-04-20'), if so all trips within or after that timestamp has their
    /// processing status reset.
    async fn check_for_out_of_order_vms_insertion(&self) -> CoreResult<()>;

    /// All vessels that have a single [`crate::Departure`] message will use the
    /// [`crate::TripAssemblerId::Ers`] trip assembler.
    /// This has the drawback where a vessel that has operated without ERS messages
    /// for some time and then starts reporting ERS, then all prior [`crate::TripAssemblerId::Landings`] trips for that vessel will be deleted
    /// and replaced with the new ERS based trips (the new ERS based trips will not cover the older landings).
    ///
    /// This method updates all vessels preferred trip assembler.
    async fn update_preferred_trip_assemblers(&self) -> CoreResult<()>;
    async fn update_trip(&self, update: TripUpdate) -> CoreResult<()>;
    async fn add_trip_set(&self, value: TripSet) -> CoreResult<()>;

    /// Trips contain different types of events which can all be scraped out of order (events
    /// that occurred in the past are added later).
    /// This presents a problem when a trip has already been generated and only the events that existed
    /// within its period at the time of creation is associated with it.
    ///
    /// Our first attempt at solving this used Postgres Materialized Views, where we created a
    /// view for trips which we refreshed each day. However, this took considerable amount of
    /// time and the entire view had to be refreshed if a single trip or out of order event was added.
    ///
    /// In our current solution we maintain a refresh boundary per vessel which indicates how far
    /// back in time we have should refresh the vessel's trips. This boundary is updated each time
    /// a new out of order event is added (if its timestamp is lower than the current boundary).
    ///
    /// This method refreshes all trips that are before or on the refresh boundary.
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
