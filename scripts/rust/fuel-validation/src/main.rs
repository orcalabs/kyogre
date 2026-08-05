use anyhow::Result;
use clap::{Parser, ValueEnum};
use fuel_validation::*;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Display, EnumIter)]
enum Vessels {
    Hera,
    Ramoen,
    Nergaard,
    Heroyfjord,
    Eros,
    SilleMarie,
}

/// Run fuel validation on vessels
#[derive(Parser, Debug)]
struct Args {
    /// Name of the vessel to run validation on
    #[arg(value_enum, short, long)]
    vessel: Option<Vessels>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let vessels = Vessels::iter().filter(|v| args.vessel.is_none_or(|a| a == *v));

    for vessel in vessels {
        println!("\n{vessel}");
        match vessel {
            Vessels::Hera => run_hera().await,
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
        }?;
    }

    Ok(())
}
