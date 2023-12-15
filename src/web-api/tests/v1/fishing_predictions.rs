use crate::v1::helper::INSIDE_HAULS_POLYGON;

use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Datelike, Utc};
use engine::*;
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use kyogre_core::*;
use web_api::routes::v1::fishing_prediction::{
    FishingSpotPrediction, FishingSpotPredictionParams, FishingWeightPrediction,
    FishingWeightPredictionParams,
};

#[tokio::test]
async fn test_fishing_spot_predictions_filters_by_model_id() {
    test(|helper, builder| async move {
        builder
            .add_ml_models(vec![
                default_fishing_spot_predictor(),
                default_fishing_spot_weather_predictor(),
            ])
            .data_start(Utc::now().with_ordinal(1).unwrap())
            .weather(5)
            .vessels(1)
            .hauls(5)
            .modify(|v| {
                v.dca.start_longitude = Some(INSIDE_HAULS_POLYGON.0);
                v.dca.start_latitude = Some(INSIDE_HAULS_POLYGON.1);
                v.dca.catch.species.living_weight = Some(100000);
                v.dca.gear.gear_group_code = GearGroup::Trawl;
            })
            .weather(5)
            .build()
            .await;

        let response = helper
            .app
            .get_all_fishing_spot_predictions(ModelId::Spot)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let predictions: Vec<FishingSpotPrediction> = response.json().await.unwrap();
        assert_eq!(predictions.len() as u32, FISHING_SPOT_PREDICTOR_NUM_DAYS);

        let response = helper
            .app
            .get_all_fishing_spot_predictions(ModelId::SpotWeather)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let predictions: Vec<FishingSpotPrediction> = response.json().await.unwrap();
        // We only have weather data for the first week
        assert_eq!(predictions.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_spot_predictions_filters_by_week_and_species_group() {
    test(|helper, builder| async move {
        builder
            .add_ml_model(default_fishing_spot_predictor())
            .vessels(1)
            .hauls(2)
            .modify_idx(|i, v| {
                v.dca.gear.gear_group_code = GearGroup::Trawl;
                v.dca.start_longitude = Some(INSIDE_HAULS_POLYGON.0);
                v.dca.start_latitude = Some(INSIDE_HAULS_POLYGON.1);
                v.dca.catch.species.living_weight = Some(100000);
                if i == 0 {
                    v.dca.catch.species.species_group_code = SpeciesGroup::Saithe;
                } else {
                    v.dca.catch.species.species_group_code = SpeciesGroup::AtlanticCod;
                }
            })
            .build()
            .await;

        let response = helper
            .app
            .get_fishing_spot_predictions(
                ModelId::Spot,
                SpeciesGroup::Saithe,
                FishingSpotPredictionParams {
                    date: Utc::now().date_naive().with_ordinal(1),
                },
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let prediction: Option<FishingSpotPrediction> = response.json().await.unwrap();
        assert!(prediction.is_some());
    })
    .await;
}

#[tokio::test]
async fn test_fishing_weight_predictions_filters_by_week_and_species_group() {
    test(|helper, builder| async move {
        builder
            .add_ml_model(default_fishing_weight_predictor())
            .vessels(1)
            .hauls(5)
            .modify(|v| {
                v.dca.catch.species.living_weight = Some(100000);
                v.dca.gear.gear_group_code = GearGroup::Trawl;
                v.dca.start_longitude = Some(INSIDE_HAULS_POLYGON.0);
                v.dca.start_latitude = Some(INSIDE_HAULS_POLYGON.1);
                v.dca.catch.species.species_group_code = SpeciesGroup::Saithe;
            })
            .build()
            .await;

        let response = helper
            .app
            .get_fishing_weight_predictions(
                ModelId::Weight,
                SpeciesGroup::Saithe,
                FishingWeightPredictionParams {
                    date: Utc::now().date_naive().with_ordinal(1),
                    limit: None,
                },
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let predictions: Vec<FishingWeightPrediction> = response.json().await.unwrap();
        assert_eq!(predictions.len() as u32, FISHING_WEIGHT_PREDICTOR_NUM_CL);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_weight_predictions_filters_by_model() {
    test(|helper, builder| async move {
        builder
            .add_ml_models(vec![
                default_fishing_weight_predictor(),
                default_fishing_weight_weather_predictor(),
            ])
            .data_start(Utc::now().with_ordinal(1).unwrap())
            .weather(5)
            .vessels(1)
            .hauls(5)
            .modify(|v| {
                v.dca.catch.species.living_weight = Some(100000);
                v.dca.gear.gear_group_code = GearGroup::Trawl;
                v.dca.start_longitude = Some(INSIDE_HAULS_POLYGON.0);
                v.dca.start_latitude = Some(INSIDE_HAULS_POLYGON.1);
            })
            .weather(5)
            .build()
            .await;

        let response = helper
            .app
            .get_all_fishing_weight_predictions(ModelId::Weight)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let predictions: Vec<FishingWeightPrediction> = response.json().await.unwrap();
        assert_eq!(
            predictions.len() as u32,
            FISHING_WEIGHT_PREDICTOR_NUM_DAYS * FISHING_WEIGHT_PREDICTOR_NUM_CL
        );

        let response = helper
            .app
            .get_all_fishing_weight_predictions(ModelId::WeightWeather)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let predictions: Vec<FishingWeightPrediction> = response.json().await.unwrap();
        assert_eq!(predictions.len() as u32, FISHING_WEIGHT_PREDICTOR_NUM_DAYS);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_weight_predictions_filters_by_limit_and_orders_by_weight_desc() {
    test(|helper, builder| async move {
        builder
            .add_ml_model(default_fishing_weight_predictor())
            .vessels(1)
            .hauls(5)
            .modify(|v| {
                v.dca.catch.species.living_weight = Some(100000);
                v.dca.gear.gear_group_code = GearGroup::Trawl;
                v.dca.catch.species.species_group_code = SpeciesGroup::Saithe;
                v.dca.start_longitude = Some(INSIDE_HAULS_POLYGON.0);
                v.dca.start_latitude = Some(INSIDE_HAULS_POLYGON.1);
            })
            .build()
            .await;

        let response = helper
            .app
            .get_fishing_weight_predictions(
                ModelId::Weight,
                SpeciesGroup::Saithe,
                FishingWeightPredictionParams {
                    date: Utc::now().date_naive().with_ordinal(1),
                    limit: Some(2),
                },
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let predictions: Vec<FishingWeightPrediction> = response.json().await.unwrap();
        assert_eq!(predictions.len() as u32, 2);
        assert!(predictions[0].weight >= predictions[1].weight);
    })
    .await;
}
