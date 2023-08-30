use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{DateTime, TimeZone, Utc};
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use kyogre_core::{FiskeridirVesselId, LandingsSorting, Ordering};
use web_api::routes::{
    utils::{DateTimeUtc, GearGroupId, SpeciesGroupId},
    v1::landing::{Landing, LandingsParams},
};

#[tokio::test]
async fn test_landings_returns_all_landings() {
    test(|helper, _builder| async move {
        let vessel_id = FiskeridirVesselId(111);
        let date = Utc.timestamp_opt(1000, 0).unwrap();

        helper
            .db
            .generate_landings(vec![
                (1, vessel_id, date),
                (2, vessel_id, date),
                (3, vessel_id, date),
            ])
            .await;

        let response = helper.app.get_landings(Default::default()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_in_specified_months() {
    test(|helper, _builder| async move {
        let mut landing1 = fiskeridir_rs::Landing::test_default(1, None);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, None);
        let landing3 = fiskeridir_rs::Landing::test_default(3, None);
        let landing4 = fiskeridir_rs::Landing::test_default(4, None);

        let month1: DateTime<Utc> = "2001-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2000-06-1T00:00:00Z".parse().unwrap();

        landing1.landing_timestamp = month1;
        landing2.landing_timestamp = month2;

        helper
            .db
            .add_landings(vec![landing1, landing2, landing3, landing4])
            .await;

        let params = LandingsParams {
            months: Some(vec![DateTimeUtc(month1), DateTimeUtc(month2)]),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_in_catch_location() {
    test(|helper, _builder| async move {
        let mut landing1 = fiskeridir_rs::Landing::test_default(1, None);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, None);
        let landing3 = fiskeridir_rs::Landing::test_default(3, None);
        let landing4 = fiskeridir_rs::Landing::test_default(4, None);

        landing1.catch_location.main_area_code = Some(9);
        landing1.catch_location.location_code = Some(5);
        landing2.catch_location.main_area_code = Some(9);
        landing2.catch_location.location_code = Some(4);

        helper
            .db
            .add_landings(vec![landing1, landing2, landing3, landing4])
            .await;

        let params = LandingsParams {
            catch_locations: Some(vec![
                "09-05".try_into().unwrap(),
                "09-04".try_into().unwrap(),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);

        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_gear_group_ids() {
    test(|helper, _builder| async move {
        let mut landing1 = fiskeridir_rs::Landing::test_default(1, None);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, None);
        let landing3 = fiskeridir_rs::Landing::test_default(3, None);
        let landing4 = fiskeridir_rs::Landing::test_default(4, None);

        landing1.gear.group = GearGroup::Not;
        landing2.gear.group = GearGroup::BurOgRuser;

        helper
            .db
            .add_landings(vec![landing1, landing2, landing3, landing4])
            .await;

        let params = LandingsParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::BurOgRuser),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_species_group_ids() {
    test(|helper, _builder| async move {
        let mut landing1 = fiskeridir_rs::Landing::test_default(1, None);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, None);
        let landing3 = fiskeridir_rs::Landing::test_default(3, None);
        let landing4 = fiskeridir_rs::Landing::test_default(4, None);

        landing1.product.species.group_code = SpeciesGroup::Blaakveite;
        landing2.product.species.group_code = SpeciesGroup::Uer;

        helper
            .db
            .add_landings(vec![landing1, landing2, landing3, landing4])
            .await;

        let params = LandingsParams {
            species_group_ids: Some(vec![
                SpeciesGroupId(SpeciesGroup::Blaakveite),
                SpeciesGroupId(SpeciesGroup::Uer),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_vessel_length_ranges() {
    test(|helper, _builder| async move {
        let mut landing1 = fiskeridir_rs::Landing::test_default(1, None);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, None);
        let landing3 = fiskeridir_rs::Landing::test_default(3, None);
        let landing4 = fiskeridir_rs::Landing::test_default(4, None);

        landing1.vessel.length = Some(9.);
        landing2.vessel.length = Some(12.);

        helper
            .db
            .add_landings(vec![landing1, landing2, landing3, landing4])
            .await;

        let params = LandingsParams {
            vessel_length_ranges: Some(vec!["(,10)".parse().unwrap(), "[10,15)".parse().unwrap()]),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_fiskeridir_vessel_ids() {
    test(|helper, _builder| async move {
        let landing1 = fiskeridir_rs::Landing::test_default(1, Some(1));
        let landing2 = fiskeridir_rs::Landing::test_default(2, Some(2));
        let landing3 = fiskeridir_rs::Landing::test_default(3, None);
        let landing4 = fiskeridir_rs::Landing::test_default(4, None);

        helper
            .db
            .add_landings(vec![landing1, landing2, landing3, landing4])
            .await;

        let params = LandingsParams {
            fiskeridir_vessel_ids: Some(vec![FiskeridirVesselId(1), FiskeridirVesselId(2)]),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_landings_sorts_by_landing_timestamp() {
    test(|helper, _builder| async move {
        let mut expected = vec![
            fiskeridir_rs::Landing::test_default(1, None),
            fiskeridir_rs::Landing::test_default(2, None),
            fiskeridir_rs::Landing::test_default(3, None),
            fiskeridir_rs::Landing::test_default(4, None),
        ];

        expected[0].landing_timestamp = Utc.timestamp_opt(1000, 0).unwrap();
        expected[1].landing_timestamp = Utc.timestamp_opt(2000, 0).unwrap();
        expected[2].landing_timestamp = Utc.timestamp_opt(3000, 0).unwrap();
        expected[3].landing_timestamp = Utc.timestamp_opt(4000, 0).unwrap();

        helper.db.add_landings(expected.clone()).await;

        let params = LandingsParams {
            sorting: Some(LandingsSorting::LandingTimestamp),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(
            landings[0].landing_timestamp.timestamp_millis(),
            expected[0].landing_timestamp.timestamp_millis()
        );
        assert_eq!(
            landings[1].landing_timestamp.timestamp_millis(),
            expected[1].landing_timestamp.timestamp_millis()
        );
        assert_eq!(
            landings[2].landing_timestamp.timestamp_millis(),
            expected[2].landing_timestamp.timestamp_millis()
        );
        assert_eq!(
            landings[3].landing_timestamp.timestamp_millis(),
            expected[3].landing_timestamp.timestamp_millis()
        );
    })
    .await;
}

#[tokio::test]
async fn test_landings_sorts_by_weight() {
    test(|helper, _builder| async move {
        let mut expected = vec![
            fiskeridir_rs::Landing::test_default(1, None),
            fiskeridir_rs::Landing::test_default(2, None),
            fiskeridir_rs::Landing::test_default(3, None),
            fiskeridir_rs::Landing::test_default(4, None),
        ];

        expected[0].product.living_weight = Some(100.0);
        expected[1].product.living_weight = Some(200.0);
        expected[2].product.living_weight = Some(300.0);
        expected[3].product.living_weight = Some(400.0);

        helper.db.add_landings(expected.clone()).await;

        let params = LandingsParams {
            sorting: Some(LandingsSorting::LivingWeight),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(
            landings[0].total_living_weight as u32,
            expected[0].product.living_weight.unwrap() as u32,
        );
        assert_eq!(
            landings[1].total_living_weight as u32,
            expected[1].product.living_weight.unwrap() as u32,
        );
        assert_eq!(
            landings[2].total_living_weight as u32,
            expected[2].product.living_weight.unwrap() as u32,
        );
        assert_eq!(
            landings[3].total_living_weight as u32,
            expected[3].product.living_weight.unwrap() as u32,
        );
    })
    .await;
}
