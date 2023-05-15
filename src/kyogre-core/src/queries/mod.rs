use serde::Deserialize;

mod fishing_facility;
mod haul;
mod pagination;

pub use fishing_facility::*;
pub use haul::*;
pub use pagination::*;

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Deserialize, Debug, Clone, Copy, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Ordering {
    #[serde(alias = "asc", alias = "Asc", alias = "ascending", alias = "Ascending")]
    Asc = 1,
    #[serde(
        alias = "desc",
        alias = "Desc",
        alias = "Descending",
        alias = "descending"
    )]
    Desc = 2,
}
