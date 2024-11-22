use chrono::NaiveDate;
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{CatchLocationId, HaulId, ModelId};
use unnest_insert::UnnestInsert;

use crate::queries::{type_to_i32, type_to_i64};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fishing_spot_predictions",
    conflict = "ml_model_id, species_group_id, date"
)]
pub struct NewFishingSpotPrediction {
    #[unnest_insert(update)]
    pub latitude: f64,
    #[unnest_insert(update)]
    pub longitude: f64,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub species_group_id: SpeciesGroup,
    pub date: NaiveDate,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub ml_model_id: ModelId,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fishing_weight_predictions",
    conflict = "ml_model_id, catch_location_id, species_group_id, date"
)]
pub struct NewFishingWeightPrediction {
    #[unnest_insert(update)]
    pub weight: f64,
    pub catch_location_id: String,
    pub species_group_id: i32,
    pub date: NaiveDate,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub ml_model_id: ModelId,
}

#[derive(Debug, Clone)]
pub struct WeightPredictorTrainingData {
    pub haul_id: HaulId,
    pub weight: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub catch_location_area_id: i32,
    pub catch_location_main_area_id: i32,
    pub species: SpeciesGroup,
    pub date: NaiveDate,
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
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub ml_model_id: ModelId,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i64")]
    pub haul_id: HaulId,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub species_group_id: SpeciesGroup,
    pub catch_location_id: String,
}

impl From<WeightPredictorTrainingData> for kyogre_core::WeightPredictorTrainingData {
    fn from(v: WeightPredictorTrainingData) -> Self {
        let WeightPredictorTrainingData {
            haul_id,
            weight,
            latitude,
            longitude,
            catch_location_area_id,
            catch_location_main_area_id,
            species,
            date,
            wind_speed_10m,
            wind_direction_10m,
            air_temperature_2m,
            relative_humidity_2m,
            air_pressure_at_sea_level,
            precipitation_amount,
            cloud_area_fraction,
        } = v;

        Self {
            weight,
            latitude,
            longitude,
            catch_location: CatchLocationId::new(
                catch_location_main_area_id,
                catch_location_area_id,
            ),
            species,
            date,
            haul_id,
            wind_speed_10m,
            wind_direction_10m,
            air_temperature_2m,
            relative_humidity_2m,
            air_pressure_at_sea_level,
            precipitation_amount,
            cloud_area_fraction,
        }
    }
}

impl From<kyogre_core::NewFishingSpotPrediction> for NewFishingSpotPrediction {
    fn from(value: kyogre_core::NewFishingSpotPrediction) -> Self {
        let kyogre_core::NewFishingSpotPrediction {
            latitude,
            longitude,
            species,
            model,
            date,
        } = value;

        Self {
            latitude,
            longitude,
            date,
            species_group_id: species,
            ml_model_id: model,
        }
    }
}

impl From<kyogre_core::NewFishingWeightPrediction> for NewFishingWeightPrediction {
    fn from(value: kyogre_core::NewFishingWeightPrediction) -> Self {
        let kyogre_core::NewFishingWeightPrediction {
            model,
            catch_location_id,
            weight,
            species,
            date,
        } = value;

        Self {
            species_group_id: species as i32,
            weight,
            catch_location_id: catch_location_id.into_inner(),
            ml_model_id: model,
            date,
        }
    }
}
