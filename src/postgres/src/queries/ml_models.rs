use crate::models::{
    FishingSpotTrainingData, FishingWeightPrediction, MLTrainingLog, NewFishingSpotPrediction,
    NewFishingWeightPrediction, WeightPredictorTrainingData,
};
use crate::{error::PostgresError, PostgresAdapter};
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::{FishingSpotPrediction, ModelId, TrainingHaul, WeatherData};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn commit_hauls_training_impl(
        &self,
        model_id: ModelId,
        hauls: Vec<TrainingHaul>,
    ) -> Result<(), PostgresError> {
        let insert: Vec<MLTrainingLog> = hauls
            .into_iter()
            .map(|v| MLTrainingLog {
                ml_model_id: model_id,
                haul_id: v.haul_id.0,
                species_group_id: v.species,
                catch_location_id: v.catch_location_id.into_inner(),
            })
            .collect();

        MLTrainingLog::unnest_insert(insert, &self.pool)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn save_model_impl(
        &self,
        model_id: ModelId,
        model: &[u8],
    ) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
UPDATE ml_models
SET
    model = $1
WHERE
    ml_model_id = $2
            "#,
            model,
            model_id as i32
        )
        .execute(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
    pub(crate) async fn model_impl(&self, model_id: ModelId) -> Result<Vec<u8>, PostgresError> {
        sqlx::query!(
            r#"
SELECT
    model
FROM
    ml_models
WHERE
    ml_model_id = $1
            "#,
            model_id as i32
        )
        .fetch_one(&self.pool)
        .await
        .change_context(PostgresError::Query)
        .map(|v| v.model.unwrap_or_default())
    }
    pub(crate) async fn existing_fishing_weight_predictions_impl(
        &self,
        model_id: ModelId,
        year: u32,
    ) -> Result<Vec<FishingWeightPrediction>, PostgresError> {
        sqlx::query_as!(
            FishingWeightPrediction,
            r#"
SELECT
    catch_location_id,
    week,
    species_group_id AS "species_group_id: SpeciesGroup",
    weight,
    "year"
FROM
    fishing_weight_predictions
WHERE
    "year" = $1
    AND ml_model_id = $2
            "#,
            year as i32,
            model_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn existing_fishing_spot_predictions_impl(
        &self,
        model_id: ModelId,
        year: u32,
    ) -> Result<Vec<FishingSpotPrediction>, PostgresError> {
        sqlx::query_as!(
            FishingSpotPrediction,
            r#"
SELECT
    latitude,
    longitude,
    week,
    species_group_id AS "species_group_id: SpeciesGroup",
    "year"
FROM
    fishing_spot_predictions
WHERE
    "year" = $1
    AND ml_model_id = $2
            "#,
            year as i32,
            model_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn add_fishing_spot_predictions_impl(
        &self,
        predictions: Vec<kyogre_core::NewFishingSpotPrediction>,
    ) -> Result<(), PostgresError> {
        let predictions: Vec<NewFishingSpotPrediction> = predictions
            .into_iter()
            .map(NewFishingSpotPrediction::from)
            .collect();

        NewFishingSpotPrediction::unnest_insert(predictions, &self.pool)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_weight_predictions_impl(
        &self,
        predictions: Vec<kyogre_core::NewFishingWeightPrediction>,
    ) -> Result<(), PostgresError> {
        let predictions: Vec<NewFishingWeightPrediction> = predictions
            .into_iter()
            .map(NewFishingWeightPrediction::from)
            .collect();

        NewFishingWeightPrediction::unnest_insert(predictions, &self.pool)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn fishing_weight_predictor_training_data_impl(
        &self,
        model_id: ModelId,
        weather_data: WeatherData,
        limit: Option<u32>,
    ) -> Result<Vec<WeightPredictorTrainingData>, PostgresError> {
        let require_weather = match weather_data {
            WeatherData::Require => false,
            WeatherData::Optional => true,
        };

        sqlx::query_as!(
            WeightPredictorTrainingData,
            r#"
SELECT
    cl.longitude AS "longitude!",
    cl.latitude AS "latitude!",
    cl.catch_area_id AS "catch_location_area_id!",
    cl.catch_main_area_id AS "catch_location_main_area_id!",
    (DATE_PART('week', h.start_timestamp))::INT AS "week!",
    hm.living_weight AS "weight",
    hm.species_group_id AS "species: SpeciesGroup",
    hm.haul_id,
    h.wind_speed_10m::DOUBLE PRECISION,
    h.wind_direction_10m::DOUBLE PRECISION,
    h.air_temperature_2m::DOUBLE PRECISION,
    h.relative_humidity_2m::DOUBLE PRECISION,
    h.air_pressure_at_sea_level::DOUBLE PRECISION,
    h.precipitation_amount::DOUBLE PRECISION,
    h.cloud_area_fraction::DOUBLE PRECISION
FROM
    hauls_matrix hm
    INNER JOIN hauls h ON hm.haul_id = h.haul_id
    INNER JOIN catch_locations cl ON cl.catch_location_id = hm.catch_location
    LEFT JOIN ml_hauls_training_log m ON m.ml_model_id = $2
    AND hm.haul_id = m.haul_id
    AND hm.species_group_id = m.species_group_id
    AND hm.catch_location = m.catch_location_id
WHERE
    (h.stop_timestamp - h.start_timestamp) < INTERVAL '2 day'
    AND hm.gear_group_id = $1
    AND m.haul_id IS NULL
    AND (
        (
            h.air_temperature_2m IS NOT NULL
            AND h.relative_humidity_2m IS NOT NULL
            AND h.air_pressure_at_sea_level IS NOT NULL
            AND h.wind_direction_10m IS NOT NULL
            AND h.precipitation_amount IS NOT NULL
            AND h.cloud_area_fraction IS NOT NULL
        )
        OR $3
    )
ORDER BY
    h.start_timestamp
LIMIT
    $4
            "#,
            GearGroup::Traal as i32,
            model_id as i32,
            require_weather,
            limit.map(|v| v as i64)
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn fishing_spot_predictor_training_data_impl(
        &self,
        model_id: ModelId,
        limit: Option<u32>,
    ) -> Result<Vec<FishingSpotTrainingData>, PostgresError> {
        sqlx::query_as!(
            FishingSpotTrainingData,
            r#"
WITH
    sums AS (
        SELECT DISTINCT
            ON (
                (DATE_PART('week', h.start_timestamp))::INT,
                hm.species_group_id
            ) (DATE_PART('week', h.start_timestamp))::INT AS week,
            hm.species_group_id,
            ST_X (ST_CENTROID (cl.polygon))::DECIMAL AS longitude,
            ST_Y (ST_CENTROID (cl.polygon))::DECIMAL AS latitude,
            SUM(hm.living_weight) OVER (
                PARTITION BY
                    (
                        (DATE_PART('week', h.start_timestamp))::INT,
                        hm.species_group_id,
                        hm.gear_group_id
                    )
            ) AS weight
        FROM
            hauls h
            INNER JOIN hauls_matrix hm ON h.haul_id = hm.haul_id
            INNER JOIN catch_locations cl ON cl.catch_location_id = hm.catch_location
            LEFT JOIN ml_hauls_training_log m ON h.haul_id = m.haul_id
            AND m.ml_model_id = $2
        WHERE
            (h.stop_timestamp - h.start_timestamp) < INTERVAL '2 day'
            AND h.gear_group_id = $1
            AND m.haul_id IS NULL
        ORDER BY
            week,
            hm.species_group_id,
            weight DESC
    )
SELECT
    sums.longitude AS "longitude!",
    sums.latitude AS "latitude!",
    (DATE_PART('week', h.start_timestamp))::INT AS "week!",
    (DATE_PART('isoyear', h.start_timestamp))::INT AS "year!",
    hm.species_group_id AS "species: SpeciesGroup",
    h.haul_id,
    hm.living_weight AS weight,
    hm.catch_location
FROM
    hauls_matrix hm
    INNER JOIN hauls h ON hm.haul_id = h.haul_id
    INNER JOIN catch_locations cl ON cl.catch_location_id = hm.catch_location
    INNER JOIN sums ON sums.species_group_id = hm.species_group_id
    AND sums.week = (DATE_PART('week', h.start_timestamp))::INT
    LEFT JOIN ml_hauls_training_log m ON m.ml_model_id = $2
    AND hm.haul_id = m.haul_id
    AND hm.species_group_id = m.species_group_id
    AND hm.catch_location = m.catch_location_id
WHERE
    (h.stop_timestamp - h.start_timestamp) < INTERVAL '2 day'
    AND hm.gear_group_id = $1
    AND m.haul_id IS NULL
ORDER BY
    h.start_timestamp
LIMIT
    $3
            "#,
            GearGroup::Traal as i32,
            model_id as i32,
            limit.map(|v| v as i64)
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) fn fishing_weight_predictions_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        week: u32,
        limit: u32,
    ) -> impl Stream<Item = Result<FishingWeightPrediction, PostgresError>> + '_ {
        sqlx::query_as!(
            FishingWeightPrediction,
            r#"
SELECT
    catch_location_id,
    week,
    species_group_id AS "species_group_id: SpeciesGroup",
    weight,
    "year"
FROM
    fishing_weight_predictions
WHERE
    ml_model_id = $1
    AND species_group_id = $2
    AND week = $3
ORDER BY
    weight DESC
LIMIT
    $4
            "#,
            model_id as i32,
            species as i32,
            week as i32,
            limit as i32
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn fishing_spot_prediction_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        week: u32,
    ) -> Result<Option<FishingSpotPrediction>, PostgresError> {
        sqlx::query_as!(
            FishingSpotPrediction,
            r#"
SELECT
    latitude,
    longitude,
    week,
    species_group_id AS "species_group_id: SpeciesGroup",
    "year"
FROM
    fishing_spot_predictions
WHERE
    species_group_id = $1
    AND week = $2
    AND ml_model_id = $3
            "#,
            species as i32,
            week as i32,
            model_id as i32
        )
        .fetch_optional(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) fn all_fishing_weight_predictions_impl(
        &self,
        model_id: ModelId,
    ) -> impl Stream<Item = Result<FishingWeightPrediction, PostgresError>> + '_ {
        sqlx::query_as!(
            FishingWeightPrediction,
            r#"
SELECT
    catch_location_id,
    week,
    species_group_id AS "species_group_id: SpeciesGroup",
    weight,
    "year"
FROM
    fishing_weight_predictions
WHERE
    ml_model_id = $1
            "#,
            model_id as i32
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) fn all_fishing_spot_predictions_impl(
        &self,
        model_id: ModelId,
    ) -> impl Stream<Item = Result<FishingSpotPrediction, PostgresError>> + '_ {
        sqlx::query_as!(
            FishingSpotPrediction,
            r#"
SELECT
    latitude,
    longitude,
    week,
    species_group_id AS "species_group_id: SpeciesGroup",
    "year"
FROM
    fishing_spot_predictions
WHERE
    ml_model_id = $1
            "#,
            model_id as i32
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
