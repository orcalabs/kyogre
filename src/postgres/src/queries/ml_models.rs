use crate::error::PostgresErrorWrapper;
use crate::models::{
    FishingSpotTrainingData, FishingWeightPrediction, MLTrainingLog, NewFishingSpotPrediction,
    NewFishingWeightPrediction, WeightPredictorTrainingData,
};
use crate::PostgresAdapter;
use chrono::NaiveDate;
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use futures::Stream;
use futures::TryStreamExt;
use kyogre_core::SPOT_PREDICTOR_SAMPLE_WEIGHT_LIMIT;
use kyogre_core::{FishingSpotPrediction, ModelId, TrainingHaul, WeatherData};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn commit_hauls_training_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        hauls: Vec<TrainingHaul>,
    ) -> Result<(), PostgresErrorWrapper> {
        let insert: Vec<MLTrainingLog> = hauls
            .into_iter()
            .map(|v| MLTrainingLog {
                ml_model_id: model_id,
                haul_id: v.haul_id.0,
                species_group_id: species,
                catch_location_id: v.catch_location_id.into_inner(),
            })
            .collect();

        MLTrainingLog::unnest_insert(insert, &self.pool).await?;

        Ok(())
    }

    pub(crate) async fn save_model_impl(
        &self,
        model_id: ModelId,
        model: &[u8],
        species: SpeciesGroup,
    ) -> Result<(), PostgresErrorWrapper> {
        sqlx::query!(
            r#"
UPDATE ml_models
SET
    model = $1
WHERE
    ml_model_id = $2
    AND species_group_id = $3
            "#,
            model,
            model_id as i32,
            species as i32
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn model_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
    ) -> Result<Vec<u8>, PostgresErrorWrapper> {
        let row = sqlx::query!(
            r#"
SELECT
    model
FROM
    ml_models
WHERE
    ml_model_id = $1
    AND species_group_id = $2
            "#,
            model_id as i32,
            species as i32
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.model.unwrap_or_default())
    }

    pub(crate) async fn existing_fishing_weight_predictions_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> Result<Vec<FishingWeightPrediction>, PostgresErrorWrapper> {
        let predictions = sqlx::query_as!(
            FishingWeightPrediction,
            r#"
SELECT
    catch_location_id,
    species_group_id AS "species_group_id: SpeciesGroup",
    weight,
    "date"
FROM
    fishing_weight_predictions
WHERE
    DATE_PART('year', "date") = $1
    AND ml_model_id = $2
    AND species_group_id = $3
            "#,
            year as i32,
            model_id as i32,
            species as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(predictions)
    }

    pub(crate) async fn existing_fishing_spot_predictions_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> Result<Vec<FishingSpotPrediction>, PostgresErrorWrapper> {
        let predictions = sqlx::query_as!(
            FishingSpotPrediction,
            r#"
SELECT
    latitude,
    longitude,
    "date",
    species_group_id AS "species_group_id: SpeciesGroup"
FROM
    fishing_spot_predictions
WHERE
    DATE_PART('year', "date") = $1
    AND ml_model_id = $2
    AND species_group_id = $3
            "#,
            year as i32,
            model_id as i32,
            species as i32
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(predictions)
    }

    pub(crate) async fn add_fishing_spot_predictions_impl(
        &self,
        predictions: Vec<kyogre_core::NewFishingSpotPrediction>,
    ) -> Result<(), PostgresErrorWrapper> {
        let predictions: Vec<NewFishingSpotPrediction> = predictions
            .into_iter()
            .map(NewFishingSpotPrediction::from)
            .collect();

        NewFishingSpotPrediction::unnest_insert(predictions, &self.pool).await?;

        Ok(())
    }

    pub(crate) async fn add_weight_predictions_impl(
        &self,
        predictions: Vec<kyogre_core::NewFishingWeightPrediction>,
    ) -> Result<(), PostgresErrorWrapper> {
        let predictions: Vec<NewFishingWeightPrediction> = predictions
            .into_iter()
            .map(NewFishingWeightPrediction::from)
            .collect();

        NewFishingWeightPrediction::unnest_insert(predictions, &self.pool).await?;

        Ok(())
    }

    pub(crate) async fn fishing_weight_predictor_training_data_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        weather_data: WeatherData,
        limit: Option<u32>,
        bycatch_percentage: Option<f64>,
        majority_species_group: bool,
    ) -> Result<Vec<WeightPredictorTrainingData>, PostgresErrorWrapper> {
        let require_weather = match weather_data {
            WeatherData::Require => false,
            WeatherData::Optional => true,
        };

        let data = sqlx::query_as!(
            WeightPredictorTrainingData,
            r#"
SELECT
    cl.longitude AS "longitude!",
    cl.latitude AS "latitude!",
    cl.catch_area_id AS "catch_location_area_id!",
    cl.catch_main_area_id AS "catch_location_main_area_id!",
    h.start_timestamp::DATE AS "date!",
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
    LEFT JOIN ml_hauls_training_log m ON m.ml_model_id = $1
    AND hm.haul_id = m.haul_id
    AND hm.species_group_id = m.species_group_id
    AND hm.catch_location = m.catch_location_id
WHERE
    (h.stop_timestamp - h.start_timestamp) < INTERVAL '2 day'
    AND hm.gear_group_id = $2
    AND m.haul_id IS NULL
    AND cl.hauls_polygon_overlap = TRUE
    AND hm.species_group_id = $3
    AND (
        (
            h.air_temperature_2m IS NOT NULL
            AND h.relative_humidity_2m IS NOT NULL
            AND h.air_pressure_at_sea_level IS NOT NULL
            AND h.wind_direction_10m IS NOT NULL
            AND h.precipitation_amount IS NOT NULL
            AND h.cloud_area_fraction IS NOT NULL
        )
        OR $4
    )
    AND (
        $5::DOUBLE PRECISION IS NULL
        OR hm.species_group_weight_percentage_of_haul >= $5
    )
    AND (
        $6::BOOLEAN IS FALSE
        OR hm.is_majority_species_group_of_haul = $6
    )
ORDER BY
    h.start_timestamp
LIMIT
    $7
            "#,
            model_id as i32,
            GearGroup::Trawl as i32,
            species as i32,
            require_weather,
            bycatch_percentage,
            majority_species_group,
            limit.map(|v| v as i64)
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(data)
    }

    pub(crate) async fn fishing_spot_predictor_training_data_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        limit: Option<u32>,
    ) -> Result<Vec<FishingSpotTrainingData>, PostgresErrorWrapper> {
        let data = sqlx::query_as!(
            FishingSpotTrainingData,
            r#"
WITH
    sums AS (
        SELECT DISTINCT
            ON (hm.species_group_id, h.start_timestamp::DATE) h.start_timestamp::DATE AS "date",
            hm.species_group_id,
            cl.longitude AS longitude,
            cl.latitude AS latitude,
            cl.catch_location_id,
            SUM(hm.living_weight) OVER (
                PARTITION BY
                    (
                        hm.species_group_id,
                        h.start_timestamp::DATE,
                        cl.catch_location_id
                    )
            ) AS weight
        FROM
            hauls h
            INNER JOIN hauls_matrix hm ON h.haul_id = hm.haul_id
            INNER JOIN catch_locations cl ON cl.catch_location_id = hm.catch_location
            LEFT JOIN ml_hauls_training_log m ON h.haul_id = m.haul_id
            AND hm.species_group_id = m.species_group_id
            AND hm.catch_location = m.catch_location_id
            AND m.ml_model_id = $1
        WHERE
            (h.stop_timestamp - h.start_timestamp) < INTERVAL '2 day'
            AND h.gear_group_id = $2
            AND m.haul_id IS NULL
            AND cl.hauls_polygon_overlap = TRUE
            AND hm.species_group_id = $3
        ORDER BY
            hm.species_group_id,
            h.start_timestamp::DATE,
            weight DESC
    )
SELECT
    sums.longitude AS "longitude!",
    sums.latitude AS "latitude!",
    hm.catch_location,
    h.start_timestamp::DATE AS "date!",
    hm.species_group_id AS "species: SpeciesGroup",
    h.haul_id
FROM
    hauls_matrix hm
    INNER JOIN hauls h ON hm.haul_id = h.haul_id
    INNER JOIN catch_locations cl ON cl.catch_location_id = hm.catch_location
    INNER JOIN sums ON sums.species_group_id = hm.species_group_id
    AND sums."date" = h.start_timestamp::DATE
    AND sums.weight > $4
    LEFT JOIN ml_hauls_training_log m ON m.ml_model_id = $1
    AND hm.haul_id = m.haul_id
    AND hm.species_group_id = m.species_group_id
    AND hm.catch_location = m.catch_location_id
    AND cl.hauls_polygon_overlap = TRUE
WHERE
    (h.stop_timestamp - h.start_timestamp) < INTERVAL '2 day'
    AND hm.gear_group_id = $2
    AND m.haul_id IS NULL
    AND hm.species_group_id = $3
ORDER BY
    h.start_timestamp
LIMIT
    $5
            "#,
            model_id as i32,
            GearGroup::Trawl as i32,
            species as i32,
            SPOT_PREDICTOR_SAMPLE_WEIGHT_LIMIT as i32,
            limit.map(|v| v as i64)
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(data)
    }

    pub(crate) fn fishing_weight_predictions_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
        limit: u32,
    ) -> impl Stream<Item = Result<FishingWeightPrediction, PostgresErrorWrapper>> + '_ {
        sqlx::query_as!(
            FishingWeightPrediction,
            r#"
SELECT
    catch_location_id,
    species_group_id AS "species_group_id: SpeciesGroup",
    weight,
    "date"
FROM
    fishing_weight_predictions
WHERE
    ml_model_id = $1
    AND species_group_id = $2
    AND "date" = $3
ORDER BY
    weight DESC
LIMIT
    $4
            "#,
            model_id as i32,
            species as i32,
            date,
            limit as i32
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }

    pub(crate) async fn fishing_spot_prediction_impl(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
    ) -> Result<Option<FishingSpotPrediction>, PostgresErrorWrapper> {
        let prediction = sqlx::query_as!(
            FishingSpotPrediction,
            r#"
SELECT
    latitude,
    longitude,
    species_group_id AS "species_group_id: SpeciesGroup",
    date
FROM
    fishing_spot_predictions
WHERE
    species_group_id = $1
    AND "date" = $2
    AND ml_model_id = $3
            "#,
            species as i32,
            date,
            model_id as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(prediction)
    }

    pub(crate) fn all_fishing_weight_predictions_impl(
        &self,
        model_id: ModelId,
    ) -> impl Stream<Item = Result<FishingWeightPrediction, PostgresErrorWrapper>> + '_ {
        sqlx::query_as!(
            FishingWeightPrediction,
            r#"
SELECT
    catch_location_id,
    species_group_id AS "species_group_id: SpeciesGroup",
    weight,
    "date"
FROM
    fishing_weight_predictions
WHERE
    ml_model_id = $1
            "#,
            model_id as i32
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }

    pub(crate) fn all_fishing_spot_predictions_impl(
        &self,
        model_id: ModelId,
    ) -> impl Stream<Item = Result<FishingSpotPrediction, PostgresErrorWrapper>> + '_ {
        sqlx::query_as!(
            FishingSpotPrediction,
            r#"
SELECT
    latitude,
    longitude,
    "date",
    species_group_id AS "species_group_id: SpeciesGroup"
FROM
    fishing_spot_predictions
WHERE
    ml_model_id = $1
            "#,
            model_id as i32
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }
}
