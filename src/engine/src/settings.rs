use crate::{trip_distancer::AisVms, ErsTripAssembler, LandingTripAssembler};
use config::{Config, ConfigError, File};
use kyogre_core::*;
use orca_core::{Environment, LogLevel, PsqlSettings, TelemetrySettings};
use serde::Deserialize;
use vessel_benchmark::*;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub telemetry: Option<TelemetrySettings>,
    pub postgres: PsqlSettings,
    pub environment: Environment,
    pub scraper: scraper::Config,
    pub honeycomb: Option<HoneycombApiKey>,
    pub single_state_run: Option<FisheryDiscriminants>,
}

#[derive(Debug, Deserialize)]
pub struct MatrixClientSettings {
    pub ip: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
}
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .expect("failed to parse APP_ENVIRONMENT");

        Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .add_source(config::Environment::with_prefix("KYOGRE_ENGINE").separator("__"))
            .set_override("environment", environment.as_str())?
            .build()?
            .try_deserialize()
    }

    pub fn telemetry_endpoint(&self) -> Option<String> {
        self.telemetry.as_ref().map(|t| t.endpoint())
    }

    pub fn honeycomb_api_key(&self) -> String {
        self.honeycomb.clone().unwrap().api_key
    }

    pub fn trip_assemblers(&self) -> Vec<Box<dyn TripAssembler>> {
        let landings_assembler = Box::<LandingTripAssembler>::default();
        let ers_assembler = Box::<ErsTripAssembler>::default();

        let vec = vec![
            ers_assembler as Box<dyn TripAssembler>,
            landings_assembler as Box<dyn TripAssembler>,
        ];

        vec
    }
    pub fn benchmarks(&self) -> Vec<Box<dyn VesselBenchmark>> {
        let weight_per_hour = Box::<WeightPerHour>::default();
        let weight_per_hour_day = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(1),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerHourDay
        });
        let weight_per_hour_week = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::weeks(1),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerHourWeek
        });
        let weight_per_hour_month = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(30),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerHourMonth
        });
        let weight_per_hour_year = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(365),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerHourYear
        });
        let weight_per_hour_prev_day = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(2*1),
            to: chrono::offset::Utc::now() - chrono::Duration::days(1),
            benchmark_id: VesselBenchmarkId::WeightPerHourPrevDay
        });
        let weight_per_hour_prev_week = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::weeks(2*1),
            to: chrono::offset::Utc::now() - chrono::Duration::weeks(1),
            benchmark_id: VesselBenchmarkId::WeightPerHourPrevWeek
        });
        let weight_per_hour_prev_month = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(2*30),
            to: chrono::offset::Utc::now() - chrono::Duration::days(30),
            benchmark_id: VesselBenchmarkId::WeightPerHourPrevMonth
        });
        let weight_per_hour_prev_year = Box::new(WeightPerHourInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(2*365),
            to: chrono::offset::Utc::now() - chrono::Duration::days(365),
            benchmark_id: VesselBenchmarkId::WeightPerHourPrevYear
        });
        let weight_per_distance = Box::<WeightPerDistance>::default();

        let weight_per_distance_day = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(1),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerDistanceDay
        });
        let weight_per_distance_week = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::weeks(1),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerDistanceWeek
        });
        let weight_per_distance_month = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(30),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerDistanceMonth
        });
        let weight_per_distance_year = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(365),
            to: chrono::offset::Utc::now(),
            benchmark_id: VesselBenchmarkId::WeightPerDistanceYear
        });

        let weight_per_distance_prev_day = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(2*1),
            to: chrono::offset::Utc::now() - chrono::Duration::days(1),
            benchmark_id: VesselBenchmarkId::WeightPerDistancePrevDay
        });
        let weight_per_distance_prev_week = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::weeks(2*1),
            to: chrono::offset::Utc::now() - chrono::Duration::weeks(1),
            benchmark_id: VesselBenchmarkId::WeightPerDistancePrevWeek
        });
        let weight_per_distance_prev_month = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(2*30),
            to: chrono::offset::Utc::now() - chrono::Duration::days(30),
            benchmark_id: VesselBenchmarkId::WeightPerDistancePrevMonth
        });
        let weight_per_distance_prev_year = Box::new(WeightPerDistanceInterval{
            from: chrono::offset::Utc::now() - chrono::Duration::days(2*365),
            to: chrono::offset::Utc::now() - chrono::Duration::days(365),
            benchmark_id: VesselBenchmarkId::WeightPerDistancePrevYear
        });
        
        let vec = vec![
            weight_per_distance as Box<dyn VesselBenchmark>,
            weight_per_distance_day as Box<dyn VesselBenchmark>, 
            weight_per_distance_week as Box<dyn VesselBenchmark>, 
            weight_per_distance_month as Box<dyn VesselBenchmark>, 
            weight_per_distance_year as Box<dyn VesselBenchmark>, 
            weight_per_distance_prev_day as Box<dyn VesselBenchmark>, 
            weight_per_distance_prev_week as Box<dyn VesselBenchmark>, 
            weight_per_distance_prev_month as Box<dyn VesselBenchmark>, 
            weight_per_distance_prev_year as Box<dyn VesselBenchmark>, 
            
            weight_per_hour as Box<dyn VesselBenchmark>, 
            weight_per_hour_day as Box<dyn VesselBenchmark>, 
            weight_per_hour_week as Box<dyn VesselBenchmark>, 
            weight_per_hour_month as Box<dyn VesselBenchmark>, 
            weight_per_hour_year as Box<dyn VesselBenchmark>, 
            weight_per_hour_prev_day as Box<dyn VesselBenchmark>, 
            weight_per_hour_prev_week as Box<dyn VesselBenchmark>, 
            weight_per_hour_prev_month as Box<dyn VesselBenchmark>, 
            weight_per_hour_prev_year as Box<dyn VesselBenchmark>, 
        ];

        vec
    }
    pub fn haul_distributors(&self) -> Vec<Box<dyn HaulDistributor>> {
        vec![Box::<haul_distributor::AisVms>::default()]
    }
    pub fn trip_distancer(&self) -> Box<dyn TripDistancer> {
        Box::<AisVms>::default()
    }
}
