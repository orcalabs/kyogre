use enum_index::EnumIndex;
use kyogre_core::{date_feature_matrix_index, NUM_CATCH_LOCATIONS};
use std::collections::HashMap;

use crate::v1::helper::{assert_matrix_content, sum_area};

use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{DateTime, Utc};
use fiskeridir_rs::{ErsDca, GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{ActiveHaulsFilter, FiskeridirVesselId, ScraperInboundPort};
use web_api::routes::{
    utils::{DateTimeUtc, GearGroupId, SpeciesGroupId},
    v1::haul::{Haul, HaulsGrid, HaulsMatrix, HaulsParams},
};

#[tokio::test]
async fn test_hauls_returns_all_hauls() {
    test(|helper| async move {
        helper.db.generate_ers_dca(1, None).await;
        helper.db.generate_ers_dca(2, None).await;
        helper.db.generate_ers_dca(3, None).await;

        let response = helper.app.get_hauls(Default::default()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_in_specified_months() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2001-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2000-06-1T00:00:00Z".parse().unwrap();

        ers1.start_date = Some(month1.date_naive());
        ers1.start_time = Some(month1.time());
        ers1.stop_date = Some(month1.date_naive());
        ers1.stop_time = Some(month1.time());
        ers2.start_date = Some(month2.date_naive());
        ers2.start_time = Some(month2.time());
        ers2.stop_date = Some(month2.date_naive());
        ers2.stop_time = Some(month2.time());

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            months: Some(vec![DateTimeUtc(month1), DateTimeUtc(month2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_in_catch_location() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            catch_locations: Some(vec![
                "09-05".try_into().unwrap(),
                "09-04".try_into().unwrap(),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_gear_group_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.gear.gear_group_code = GearGroup::Not;
        ers2.gear.gear_group_code = GearGroup::BurOgRuser;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::BurOgRuser),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_species_group_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers2.catch.species.species_group_code = SpeciesGroup::Uer;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            species_group_ids: Some(vec![SpeciesGroupId(301), SpeciesGroupId(302)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_vessel_length_ranges() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers2.vessel_info.vessel_length = 12.;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            vessel_length_ranges: Some(vec!["(,10)".parse().unwrap(), "[10,15)".parse().unwrap()]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_fiskeridir_vessel_ids() {
    test(|helper| async move {
        let ers1 = ErsDca::test_default(1, Some(1));
        let ers2 = ErsDca::test_default(2, Some(2));
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            fiskeridir_vessel_ids: Some(vec![FiskeridirVesselId(1), FiskeridirVesselId(2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_grid_returns_grid_for_all_hauls() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.gear.gear_group_code = GearGroup::Garn;

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.catch.species.living_weight = Some(20);
        ers2.gear.gear_group_code = GearGroup::Garn;

        let mut ers3 = ers1.clone();
        ers3.message_info.message_id = 3;

        let mut ers4 = ers2.clone();
        ers4.message_info.message_id = 4;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let response = helper.app.get_hauls_grid(Default::default()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([
                    ("09-05".try_into().unwrap(), 20),
                    ("09-04".try_into().unwrap(), 40)
                ]),
                max_weight: 40,
                min_weight: 20,
                weight_by_gear_group: HashMap::from([(GearGroup::Garn, 60)]),
                weight_by_species_group: HashMap::from([(201, 60)]),
                weight_by_vessel_length_group: HashMap::from([(
                    VesselLengthGroup::TwentyEightAndAbove,
                    60
                )])
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_hauls_grid_returns_grid_for_hauls_in_specified_month() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2001-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2000-06-1T00:00:00Z".parse().unwrap();

        ers1.start_date = Some(month1.date_naive());
        ers1.start_time = Some(month1.time());
        ers1.stop_date = Some(month1.date_naive());
        ers1.stop_time = Some(month1.time());
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.gear.gear_group_code = GearGroup::Garn;

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.start_date = Some(month2.date_naive());
        ers2.start_time = Some(month2.time());
        ers2.stop_date = Some(month2.date_naive());
        ers2.stop_time = Some(month2.time());
        ers2.catch.species.living_weight = Some(20);
        ers2.gear.gear_group_code = GearGroup::Garn;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            months: Some(vec![DateTimeUtc(month1), DateTimeUtc(month2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_grid(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([
                    ("09-05".try_into().unwrap(), 10),
                    ("09-04".try_into().unwrap(), 20)
                ]),
                max_weight: 20,
                min_weight: 10,
                weight_by_gear_group: HashMap::from([(GearGroup::Garn, 30)]),
                weight_by_species_group: HashMap::from([(201, 30)]),
                weight_by_vessel_length_group: HashMap::from([(
                    VesselLengthGroup::TwentyEightAndAbove,
                    30
                )])
            }
        );
    })
    .await;
}
#[tokio::test]
async fn test_hauls_grid_returns_grid_for_hauls_in_catch_location() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.gear.gear_group_code = GearGroup::Garn;

        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);
        ers2.gear.gear_group_code = GearGroup::Garn;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            catch_locations: Some(vec!["09-05".try_into().unwrap()]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_grid(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([("09-05".try_into().unwrap(), 30)]),
                max_weight: 30,
                min_weight: 30,
                weight_by_gear_group: HashMap::from([(GearGroup::Garn, 30)]),
                weight_by_species_group: HashMap::from([(201, 30)]),
                weight_by_vessel_length_group: HashMap::from([(
                    VesselLengthGroup::TwentyEightAndAbove,
                    30
                )])
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_hauls_grid_returns_grid_for_hauls_with_gear_group_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.gear.gear_group_code = GearGroup::Not;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.catch.species.species_fdir_code = Some(201);

        ers2.gear.gear_group_code = GearGroup::BurOgRuser;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);
        ers2.catch.species.species_fdir_code = Some(201);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::BurOgRuser),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_grid(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([("09-05".try_into().unwrap(), 30)]),
                max_weight: 30,
                min_weight: 30,
                weight_by_gear_group: HashMap::from([
                    (GearGroup::Not, 10),
                    (GearGroup::BurOgRuser, 20)
                ]),
                weight_by_species_group: HashMap::from([(201, 30)]),
                weight_by_vessel_length_group: HashMap::from([(
                    VesselLengthGroup::TwentyEightAndAbove,
                    30
                )])
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_hauls_grid_returns_grid_for_hauls_with_species_group_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.gear.gear_group_code = GearGroup::Garn;

        ers2.catch.species.species_group_code = SpeciesGroup::Uer;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);
        ers2.gear.gear_group_code = GearGroup::Garn;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            species_group_ids: Some(vec![SpeciesGroupId(301), SpeciesGroupId(302)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_grid(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([("09-05".try_into().unwrap(), 30)]),
                max_weight: 30,
                min_weight: 30,
                weight_by_gear_group: HashMap::from([(GearGroup::Garn, 30)]),
                weight_by_species_group: HashMap::from([(301, 10), (302, 20)]),
                weight_by_vessel_length_group: HashMap::from([(
                    VesselLengthGroup::TwentyEightAndAbove,
                    30
                )])
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_hauls_grid_returns_grid_for_hauls_with_vessel_length_ranges() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.gear.gear_group_code = GearGroup::Garn;

        ers2.vessel_info.vessel_length = 12.;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);
        ers2.gear.gear_group_code = GearGroup::Garn;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            vessel_length_ranges: Some(vec!["(,10)".parse().unwrap(), "[10,15)".parse().unwrap()]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_grid(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([("09-05".try_into().unwrap(), 30)]),
                max_weight: 30,
                min_weight: 30,
                weight_by_gear_group: HashMap::from([(GearGroup::Garn, 30)]),
                weight_by_species_group: HashMap::from([(201, 30)]),
                weight_by_vessel_length_group: HashMap::from([
                    (VesselLengthGroup::UnderEleven, 10),
                    (VesselLengthGroup::ElevenToFifteen, 20)
                ])
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_hauls_grid_returns_grid_for_hauls_with_fiskeridir_vessel_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, Some(1));
        let mut ers2 = ErsDca::test_default(2, Some(2));
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);
        ers1.gear.gear_group_code = GearGroup::Garn;

        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);
        ers2.gear.gear_group_code = GearGroup::Garn;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            fiskeridir_vessel_ids: Some(vec![FiskeridirVesselId(1), FiskeridirVesselId(2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_grid(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let grid: HaulsGrid = response.json().await.unwrap();

        assert_eq!(
            grid,
            HaulsGrid {
                grid: HashMap::from([("09-05".try_into().unwrap(), 30)]),
                max_weight: 30,
                min_weight: 30,
                weight_by_gear_group: HashMap::from([(GearGroup::Garn, 30)]),
                weight_by_species_group: HashMap::from([(201, 30)]),
                weight_by_vessel_length_group: HashMap::from([(
                    VesselLengthGroup::TwentyEightAndAbove,
                    30,
                )])
            }
        );
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_returns_correct_sum_for_all_hauls() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;
        let mut ers1 = ErsDca::test_default(1, Some(1));
        let mut ers2 = ErsDca::test_default(2, Some(2));

        ers1.catch.species.living_weight = Some(20);
        ers2.catch.species.living_weight = Some(40);

        ers1.start_latitude = Some(70.536);
        ers1.start_longitude = Some(21.957);
        ers2.start_latitude = Some(70.536);
        ers2.start_longitude = Some(21.957);

        helper.db.db.add_ers_dca(vec![ers1, ers2]).await.unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        assert_matrix_content(&matrix, filter, 60);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_months() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        ers1.start_date = Some(month1.date_naive());
        ers1.start_time = Some(month1.time());
        ers1.stop_date = Some(month1.date_naive());
        ers1.stop_time = Some(month1.time());
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.start_date = Some(month2.date_naive());
        ers2.start_time = Some(month2.time());
        ers2.stop_date = Some(month2.date_naive());
        ers2.stop_time = Some(month2.time());
        ers2.catch.species.living_weight = Some(20);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            months: Some(vec![DateTimeUtc(month1), DateTimeUtc(month2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_vessel_length() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::VesselLength;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.vessel_info.vessel_length = 12.;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            vessel_length_ranges: Some(vec!["(,10)".parse().unwrap(), "[10,15)".parse().unwrap()]),
            ..Default::default()
        };

        let response = helper
            .app
            .get_hauls_matrix(params, ActiveHaulsFilter::VesselLength)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_species_group() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.catch.species.species_group_code = SpeciesGroup::Uer;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            species_group_ids: Some(vec![SpeciesGroupId(301), SpeciesGroupId(302)]),
            ..Default::default()
        };

        let response = helper
            .app
            .get_hauls_matrix(params, ActiveHaulsFilter::SpeciesGroup)
            .await;

        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_gear_group() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::GearGroup;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.gear.gear_group_code = GearGroup::Not;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.gear.gear_group_code = GearGroup::Garn;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::Garn),
            ]),
            ..Default::default()
        };

        let response = helper
            .app
            .get_hauls_matrix(params, ActiveHaulsFilter::GearGroup)
            .await;

        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_fiskeridir_vessel_ids() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;

        let mut ers1 = ErsDca::test_default(1, Some(1));
        let mut ers2 = ErsDca::test_default(2, Some(2));
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            fiskeridir_vessel_ids: Some(vec![FiskeridirVesselId(1), FiskeridirVesselId(2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_date_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        ers1.start_date = Some(month1.date_naive());
        ers1.start_time = Some(month1.time());
        ers1.stop_date = Some(month1.date_naive());
        ers1.stop_time = Some(month1.time());
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.start_date = Some(month2.date_naive());
        ers2.start_time = Some(month2.time());
        ers2.stop_date = Some(month2.date_naive());
        ers2.stop_time = Some(month2.time());
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = date_feature_matrix_index(&month1);
        let y1 = date_feature_matrix_index(&month2);

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.dates, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_gear_group_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::GearGroup;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.gear.gear_group_code = GearGroup::Traal;
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.gear.gear_group_code = GearGroup::Snurrevad;
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.gear.gear_group_code = GearGroup::Not;
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.gear.gear_group_code = GearGroup::Not;
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = GearGroup::Traal.enum_index();
        let y1 = GearGroup::Snurrevad.enum_index();

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.gear_group, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_vessel_length_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::VesselLength;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.vessel_info.vessel_length = 12.;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = VesselLengthGroup::UnderEleven.enum_index();
        let y1 = VesselLengthGroup::ElevenToFifteen.enum_index();

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.length_group, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_species_group_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.catch.species.species_group_code = SpeciesGroup::Uer;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = SpeciesGroup::Blaakveite.enum_index();
        let y1 = SpeciesGroup::Uer.enum_index();

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.species_group, x0, y0, x1, y1, width), 30);
    })
    .await;
}
