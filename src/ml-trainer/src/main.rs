use clap::Parser;
use engine::{
    FishingSpotPredictor, FishingWeightPredictor, FishingWeightWeatherPredictor,
    SpotPredictorSettings, WeightPredictorSettings,
};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{MLModel, MLModelsOutbound, PredictionRange, TrainingMode};
use num_traits::FromPrimitive;
use orca_core::{PsqlLogStatements, PsqlSettings};
use postgres::PostgresAdapter;
use strum::IntoEnumIterator;

#[derive(clap::ValueEnum, Debug, Copy, Clone)]
enum ModelId {
    Spot = 1,
    Weight = 2,
    WeightWeather = 3,
    SpotWeather = 4,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum)]
    model: Option<ModelId>,

    #[arg(short, long)]
    species_group_id: Option<u32>,

    #[arg(short, long, default_value_t = 200)]
    training_rounds: u32,

    #[arg(short, long, default_value_t = 10)]
    predict_num_days: u32,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let enabled_models = args
        .model
        .map(|v| vec![kyogre_core::ModelId::from(v)])
        .unwrap_or_else(|| kyogre_core::ModelId::iter().collect());

    let single_species_mode = args
        .species_group_id
        .map(|v| SpeciesGroup::from_u32(v).unwrap());

    let adapter = PostgresAdapter::new(&PsqlSettings {
        ip: "127.0.0.0".to_string(),
        port: 5432,
        db_name: None,
        username: "postgres".to_string(),
        password: "test123".to_string(),
        max_connections: 1,
        root_cert: None,
        log_statements: PsqlLogStatements::Disable,
    })
    .await
    .unwrap();

    adapter.reset_models(&enabled_models).await.unwrap();

    let weight_settings = WeightPredictorSettings {
        running_in_test: true,
        use_gpu: true,
        training_rounds: args.training_rounds,
        predict_batch_size: 1000000,
        range: PredictionRange::DaysFromStartOfYear(args.predict_num_days),
        catch_locations: vec![],
        single_species_mode,
        training_mode: TrainingMode::Local,
        test_fraction: Some(0.2),
    };

    let mut models = Vec::new();
    for m in enabled_models {
        match m {
            kyogre_core::ModelId::Spot => {
                let model = Box::new(FishingSpotPredictor::new(SpotPredictorSettings {
                    running_in_test: true,
                    use_gpu: true,
                    training_rounds: args.training_rounds,
                    predict_batch_size: 1000000,
                    range: PredictionRange::DaysFromStartOfYear(args.predict_num_days),
                    catch_locations: vec![],
                    single_species_mode,
                    training_mode: TrainingMode::Local,
                    test_fraction: Some(0.2),
                }));
                models.push(model as Box<dyn MLModel>);
            }
            kyogre_core::ModelId::Weight => {
                let model = Box::new(FishingWeightPredictor::new(weight_settings.clone()));
                models.push(model as Box<dyn MLModel>);
            }
            kyogre_core::ModelId::WeightWeather => {
                let model = Box::new(FishingWeightWeatherPredictor::new(weight_settings.clone()));
                models.push(model as Box<dyn MLModel>);
            }
            kyogre_core::ModelId::SpotWeather => unimplemented!(),
        }
    }

    for m in models {
        let trained = m
            .train(adapter.model(m.id()).await.unwrap(), &adapter)
            .await
            .unwrap();
        m.predict(&trained, &adapter).await.unwrap();
    }
}

impl From<ModelId> for kyogre_core::ModelId {
    fn from(value: ModelId) -> Self {
        match value {
            ModelId::Spot => kyogre_core::ModelId::Spot,
            ModelId::Weight => kyogre_core::ModelId::Weight,
            ModelId::WeightWeather => kyogre_core::ModelId::WeightWeather,
            ModelId::SpotWeather => kyogre_core::ModelId::SpotWeather,
        }
    }
}
