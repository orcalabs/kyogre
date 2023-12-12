use clap::{Args, Parser, Subcommand};
use engine::{
    FishingSpotPredictor, FishingSpotWeatherPredictor, FishingWeightPredictor,
    FishingWeightWeatherPredictor, SpotPredictorSettings, WeightPredictorSettings,
};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{MLModel, MLModelsOutbound, PredictionRange, TrainingMode, ML_SPECIES_GROUPS};
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

#[derive(clap::ValueEnum, Debug, Copy, Clone)]
enum Mode {
    Train,
    Full,
}

#[derive(Debug, Clone)]
enum SpeciesMode {
    All,
    Specific(Vec<SpeciesGroup>),
}

pub struct Output {
    model: kyogre_core::ModelId,
    species: SpeciesGroup,
    training_score: f64,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct BaseArgs {
    #[arg(long, value_enum)]
    model: Option<ModelId>,

    #[arg(short, long, value_delimiter = ' ')]
    species: Option<Vec<u32>>,

    #[arg(short, long, default_value_t = 200)]
    training_rounds: u32,

    #[arg(short, long, default_value_t = false)]
    majority_species_group: bool,

    #[arg(short, long)]
    bycatch: Option<f64>,
}

#[derive(Args, Debug)]
struct InteractiveArgs {
    #[arg(long, value_enum, default_value_t = Mode::Full)]
    mode: Mode,

    #[arg(short, long, default_value_t = 10)]
    predict_num_days: u32,
}

#[derive(Args, Debug)]
struct ExperimentArgs {}

struct Experiment {
    majority_species_group: bool,
    species: SpeciesGroup,
    bycatch: Option<f64>,
    model: Box<dyn MLModel>,
}

impl std::fmt::Display for Experiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "model: {}\n species: {}\n majority_species_group: {}\n bycatch_percentage: {}",
            self.model.id(),
            self.species.norwegian_name(),
            self.majority_species_group,
            self.bycatch.unwrap_or_default()
        ))
    }
}

#[derive(Subcommand)]
enum Commands {
    Interactive {
        #[command(flatten)]
        base: BaseArgs,
        #[command(flatten)]
        args: InteractiveArgs,
    },
    Experiment {
        #[command(flatten)]
        base: BaseArgs,
        #[command(flatten)]
        args: ExperimentArgs,
    },
}

impl Commands {
    fn species(&self) -> Vec<SpeciesGroup> {
        match self {
            Commands::Interactive { base, args: _ } | Commands::Experiment { base, args: _ } => {
                base.species
                    .clone()
                    .map(|s| {
                        s.into_iter()
                            .map(|v| SpeciesGroup::from_u32(v).unwrap())
                            .collect()
                    })
                    .unwrap_or_default()
            }
        }
    }
    fn enabled_models(&self) -> Vec<kyogre_core::ModelId> {
        match self {
            Commands::Interactive { base, args: _ } | Commands::Experiment { base, args: _ } => {
                base.model
                    .map(|v| vec![kyogre_core::ModelId::from(v)])
                    .unwrap_or_else(|| {
                        kyogre_core::ModelId::iter()
                            .filter(|v| !matches!(v, kyogre_core::ModelId::SpotWeather))
                            .collect()
                    })
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

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

    let species = cli.command.species();
    let models = cli.command.enabled_models();

    match cli.command {
        Commands::Interactive { base, args } => {
            run_interactive(args, base, adapter, models, species).await
        }
        Commands::Experiment { base, args } => {
            run_experiment(args, base, adapter, models, species).await
        }
    }
}

async fn run_experiment(
    _args: ExperimentArgs,
    base_args: BaseArgs,
    adapter: PostgresAdapter,
    enabled_models: Vec<kyogre_core::ModelId>,
    species: Vec<SpeciesGroup>,
) {
    let species_mode = match species.is_empty() {
        true => SpeciesMode::All,
        false => SpeciesMode::Specific(species),
    };

    let mut experiments = Vec::new();

    let weight_settings = WeightPredictorSettings {
        running_in_test: true,
        use_gpu: true,
        training_rounds: 1,
        predict_batch_size: 1000000,
        range: PredictionRange::DaysFromStartOfYear(0),
        catch_locations: vec![],
        training_mode: TrainingMode::Local,
        test_fraction: Some(0.2),
        bycatch_percentage: base_args.bycatch,
        majority_species_group: base_args.majority_species_group,
    };

    let spot_settings = SpotPredictorSettings {
        running_in_test: true,
        use_gpu: true,
        training_rounds: base_args.training_rounds,
        predict_batch_size: 1000000,
        range: PredictionRange::DaysFromStartOfYear(0),
        catch_locations: vec![],
        training_mode: TrainingMode::Local,
        test_fraction: Some(0.2),
    };

    match species_mode {
        SpeciesMode::All => {
            for species in ML_SPECIES_GROUPS {
                for m in &enabled_models {
                    match m {
                        kyogre_core::ModelId::Spot => {
                            let model = Box::new(FishingSpotPredictor::new(spot_settings.clone()));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: *species,
                            });
                        }
                        kyogre_core::ModelId::Weight => {
                            let model =
                                Box::new(FishingWeightPredictor::new(weight_settings.clone()));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: *species,
                            });
                        }
                        kyogre_core::ModelId::WeightWeather => {
                            let model = Box::new(FishingWeightWeatherPredictor::new(
                                weight_settings.clone(),
                            ));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: *species,
                            });
                        }
                        kyogre_core::ModelId::SpotWeather => unimplemented!(),
                    }
                }
            }
        }
        SpeciesMode::Specific(species) => {
            for s in species {
                for m in &enabled_models {
                    match m {
                        kyogre_core::ModelId::Spot => {
                            let settings = spot_settings.clone();
                            let model = Box::new(FishingSpotPredictor::new(settings));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: s,
                            });
                        }
                        kyogre_core::ModelId::Weight => {
                            let settings = weight_settings.clone();
                            let model = Box::new(FishingWeightPredictor::new(settings));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: s,
                            });
                        }
                        kyogre_core::ModelId::WeightWeather => {
                            let settings = weight_settings.clone();
                            let model = Box::new(FishingWeightWeatherPredictor::new(settings));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: s,
                            });
                        }
                        kyogre_core::ModelId::SpotWeather => {
                            let settings = spot_settings.clone();
                            let model = Box::new(FishingSpotWeatherPredictor::new(settings));
                            experiments.push(Experiment {
                                majority_species_group: base_args.majority_species_group,
                                bycatch: base_args.bycatch,
                                model,
                                species: s,
                            });
                        }
                    }
                }
            }
        }
    };

    let mut outputs: Vec<(Experiment, f64)> = Vec::new();
    for e in experiments {
        let output = e
            .model
            .train(
                adapter.model(e.model.id(), e.species).await.unwrap(),
                e.species,
                &adapter,
            )
            .await
            .unwrap();
        outputs.push((e, output.best_score.unwrap()));
    }

    outputs.sort_by_key(|v| v.0.model.id());

    for o in outputs {
        println!("Experiment:\n {}\n score: {}", o.0, o.1);
    }
}

async fn run_interactive(
    args: InteractiveArgs,
    base_args: BaseArgs,
    adapter: PostgresAdapter,
    enabled_models: Vec<kyogre_core::ModelId>,
    species: Vec<SpeciesGroup>,
) {
    let species_mode = match species.is_empty() {
        true => SpeciesMode::All,
        false => SpeciesMode::Specific(species),
    };

    adapter.reset_models(&enabled_models).await.unwrap();

    let mut outputs: Vec<Output> = Vec::new();

    match species_mode {
        SpeciesMode::All => {
            for s in ML_SPECIES_GROUPS {
                let mut out =
                    run_models_on_species(&adapter, &args, &base_args, &enabled_models, *s).await;
                outputs.append(&mut out);
            }
        }
        SpeciesMode::Specific(species) => {
            for s in species {
                let mut out =
                    run_models_on_species(&adapter, &args, &base_args, &enabled_models, s).await;
                outputs.append(&mut out);
            }
        }
    };

    outputs.sort_by_key(|v| v.model);
    for o in outputs {
        println!(
            "model: {}, species: {}, score: {}",
            o.model,
            o.species.norwegian_name(),
            o.training_score
        );
    }
}

async fn run_models_on_species(
    adapter: &PostgresAdapter,
    args: &InteractiveArgs,
    base_args: &BaseArgs,
    enabled_models: &[kyogre_core::ModelId],
    species: SpeciesGroup,
) -> Vec<Output> {
    let spot_settings = SpotPredictorSettings {
        running_in_test: true,
        use_gpu: true,
        training_rounds: base_args.training_rounds,
        predict_batch_size: 100000,
        range: PredictionRange::DaysFromStartOfYear(args.predict_num_days),
        catch_locations: vec![],
        training_mode: TrainingMode::Local,
        test_fraction: Some(0.2),
    };

    let weight_settings = WeightPredictorSettings {
        running_in_test: true,
        use_gpu: true,
        training_rounds: base_args.training_rounds,
        predict_batch_size: 100000,
        range: PredictionRange::DaysFromStartOfYear(args.predict_num_days),
        catch_locations: vec![],
        training_mode: TrainingMode::Local,
        test_fraction: Some(0.2),
        bycatch_percentage: base_args.bycatch,
        majority_species_group: base_args.majority_species_group,
    };

    let mut models = Vec::new();
    for m in enabled_models {
        match m {
            kyogre_core::ModelId::Spot => {
                let model = Box::new(FishingSpotPredictor::new(spot_settings.clone()));
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
            kyogre_core::ModelId::SpotWeather => {
                let model = Box::new(FishingSpotWeatherPredictor::new(spot_settings.clone()));
                models.push(model as Box<dyn MLModel>);
            }
        }
    }

    let mut outputs: Vec<Output> = Vec::new();
    for m in models {
        let score = match args.mode {
            Mode::Train => {
                let output = m
                    .train(
                        adapter.model(m.id(), species).await.unwrap(),
                        species,
                        adapter,
                    )
                    .await
                    .unwrap();

                output.best_score.unwrap()
            }
            Mode::Full => {
                let output = m
                    .train(
                        adapter.model(m.id(), species).await.unwrap(),
                        species,
                        adapter,
                    )
                    .await
                    .unwrap();
                m.predict(&output.model, species, adapter).await.unwrap();
                output.best_score.unwrap()
            }
        };
        outputs.push(Output {
            model: m.id(),
            species,
            training_score: score,
        });
    }

    outputs
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
