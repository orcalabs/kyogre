use std::collections::HashMap;

use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{DateTime, Utc};
use fiskeridir_rs::{ErsDca, GearGroup, VesselLengthGroup};
use kyogre_core::ScraperInboundPort;
use web_api::routes::{
    utils::{DateTimeUtc, GearGroupId, SpeciesGroupId},
    v1::haul::{Haul, HaulsGrid, HaulsParams},
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

        ers1.gear.gear_group_code = Some(1);
        ers2.gear.gear_group_code = Some(5);

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
                GearGroupId(GearGroup::Traal),
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

        ers1.catch.species.species_group_code = Some(301);
        ers2.catch.species.species_group_code = Some(302);

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
async fn test_hauls_grid_returns_grid_for_all_hauls() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.catch.species.living_weight = Some(20);

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

        ers1.gear.gear_group_code = Some(1);
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.gear.gear_group_code = Some(5);
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
                GearGroupId(GearGroup::Traal),
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
                weight_by_gear_group: HashMap::from([(GearGroup::Not, 10), (GearGroup::Traal, 20)]),
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

        ers1.catch.species.species_group_code = Some(301);
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.catch.species.species_group_code = Some(302);
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
