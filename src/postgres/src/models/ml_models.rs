use error_stack::{Report, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::CatchLocationId;
use unnest_insert::UnnestInsert;

use crate::error::PostgresError;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fishing_spot_predictions",
    conflict = "species_group_id, week, year"
)]
pub struct NewFishingSpotPrediction {
    #[unnest_insert(update)]
    pub latitude: f64,
    #[unnest_insert(update)]
    pub longitude: f64,
    pub species_group_id: i32,
    pub week: i32,
    pub year: i32,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fishing_weight_predictions",
    conflict = "catch_location_id, species_group_id, week, year"
)]
pub struct NewFishingWeightPrediction {
    #[unnest_insert(update)]
    pub weight: f64,
    pub catch_location_id: String,
    pub species_group_id: i32,
    pub week: i32,
    pub year: i32,
}

#[derive(Debug, Clone)]
pub struct FishingWeightPrediction {
    pub catch_location_id: String,
    pub weight: f64,
    pub species_group_id: SpeciesGroup,
    pub week: i32,
    pub year: i32,
}

#[derive(Debug, Clone)]
pub struct WeightPredictorTrainingData {
    pub haul_id: i64,
    pub weight: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub catch_location_area_id: i32,
    pub catch_location_main_area_id: i32,
    pub species: SpeciesGroup,
    pub week: i32,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ml_hauls_training_log")]
pub struct MLTrainingLog {
    pub ml_model_id: i32,
    pub haul_id: i64,
}

impl TryFrom<FishingWeightPrediction> for kyogre_core::FishingWeightPrediction {
    type Error = Report<PostgresError>;

    fn try_from(value: FishingWeightPrediction) -> std::result::Result<Self, Self::Error> {
        let catch_location_id = CatchLocationId::try_from(value.catch_location_id.as_str())
            .change_context(PostgresError::DataConversion)?;

        Ok(Self {
            catch_location_id,
            weight: value.weight,
            species_group_id: value.species_group_id,
            week: value.week as u32,
            year: value.year as u32,
        })
    }
}

impl From<WeightPredictorTrainingData> for kyogre_core::WeightPredictorTrainingData {
    fn from(value: WeightPredictorTrainingData) -> Self {
        Self {
            weight: value.weight,
            latitude: value.latitude,
            longitude: value.longitude,
            catch_location: CatchLocationId::new(
                value.catch_location_main_area_id,
                value.catch_location_area_id,
            ),
            species: value.species,
            week: value.week,
            haul_id: value.haul_id,
        }
    }
}

impl From<kyogre_core::NewFishingSpotPrediction> for NewFishingSpotPrediction {
    fn from(value: kyogre_core::NewFishingSpotPrediction) -> Self {
        Self {
            latitude: value.latitude,
            longitude: value.longitude,
            species_group_id: value.species.into(),
            week: value.week as i32,
            year: value.year as i32,
        }
    }
}

impl From<kyogre_core::NewFishingWeightPrediction> for NewFishingWeightPrediction {
    fn from(value: kyogre_core::NewFishingWeightPrediction) -> Self {
        Self {
            species_group_id: value.species as i32,
            week: value.week as i32,
            weight: value.weight,
            catch_location_id: value.catch_location_id.into_inner(),
            year: value.year as i32,
        }
    }
}
