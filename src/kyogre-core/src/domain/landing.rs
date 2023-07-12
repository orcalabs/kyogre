use serde::{Deserialize, Serialize};

pub static LANDING_OLDEST_DATA_MONTHS: usize = 1999 * 12;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct LandingMatrix {
    pub dates: Vec<f64>,
    pub length_group: Vec<f64>,
    pub gear_group: Vec<f64>,
    pub species_group: Vec<f64>,
}
