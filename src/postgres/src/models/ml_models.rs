use error_stack::{Report, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{CatchLocationId, ModelId};
use unnest_insert::UnnestInsert;

use crate::error::PostgresError;
use crate::queries::enum_to_i32;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fishing_spot_predictions",
    conflict = "ml_model_id, species_group_id, week, year"
)]
pub struct NewFishingSpotPrediction {
    #[unnest_insert(update)]
    pub latitude: f64,
    #[unnest_insert(update)]
    pub longitude: f64,
    pub species_group_id: i32,
    pub week: i32,
    pub year: i32,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub ml_model_id: ModelId,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fishing_weight_predictions",
    conflict = "ml_model_id, catch_location_id, species_group_id, week, year"
)]
pub struct NewFishingWeightPrediction {
    #[unnest_insert(update)]
    pub weight: f64,
    pub catch_location_id: String,
    pub species_group_id: i32,
    pub week: i32,
    pub year: i32,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub ml_model_id: ModelId,
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
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ml_hauls_training_log")]
pub struct MLTrainingLog {
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub ml_model_id: ModelId,
    pub haul_id: i64,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub species_group_id: SpeciesGroup,
    pub catch_location_id: String,
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
    fn from(v: WeightPredictorTrainingData) -> Self {
        Self {
            weight: v.weight,
            latitude: v.latitude,
            longitude: v.longitude,
            catch_location: CatchLocationId::new(
                v.catch_location_main_area_id,
                v.catch_location_area_id,
            ),
            species: v.species,
            week: v.week,
            haul_id: v.haul_id,
            wind_speed_10m: v.wind_speed_10m,
            wind_direction_10m: v.wind_speed_10m,
            air_temperature_2m: v.air_temperature_2m,
            relative_humidity_2m: v.relative_humidity_2m,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level,
            precipitation_amount: v.precipitation_amount,
            cloud_area_fraction: v.cloud_area_fraction,
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
            ml_model_id: value.model,
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
            ml_model_id: value.model,
        }
    }
}