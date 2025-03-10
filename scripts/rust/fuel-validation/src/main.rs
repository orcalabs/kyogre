use anyhow::Result;
use clap::{Parser, ValueEnum};
use fuel_validation::*;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Vessels {
    Ramoen,
    #[default]
    Nergaard,
    Heroyfjord,
    Eros,
    SilleMarie,
}

/// Run fuel validation on vessels
#[derive(Parser, Debug)]
struct Args {
    /// Name of the vessel to run validation on
    #[arg(value_enum, short, long, default_value_t = Vessels::default())]
    vessel: Vessels,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.vessel {
        Vessels::Ramoen => run_ramoen().await,
        Vessels::Nergaard => run_nergard().await,
        Vessels::Heroyfjord => {
            let (name, vessel_id, bytes) = (
                "HERØYFJORD",
                2021117460,
                include_bytes!("../Herøyfjord oljeforbruk 2022-24.xlsx"),
            );
            run_heroyfjord_eros(bytes, vessel_id, name).await
        }
        Vessels::Eros => {
            let (name, vessel_id, bytes) = (
                "TUROVERSIKT EROS",
                2013060592,
                include_bytes!("../EROS oljeforbruk 2022 - 2024.xlsx"),
            );
            run_heroyfjord_eros(bytes, vessel_id, name).await
        }
        Vessels::SilleMarie => run_sille_marie().await,
    }
}
