use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{DateTime, Utc};
use engine::*;
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use kyogre_core::{FiskeridirVesselId, LandingsSorting, Ordering};
use web_api::routes::{
    utils::{DateTimeUtc, GearGroupId, SpeciesGroupId},
    v1::landing::{Landing, LandingsParams},
};

#[tokio::test]
async fn test_landings_returns_all_landings() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).landings(3).build().await;

        let response = helper
            .app
            .get_landings(LandingsParams {
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 3);
        assert_eq!(landings, state.landings);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_in_specified_months() {
    test(|helper, builder| async move {
        let month1: DateTime<Utc> = "2000-06-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2001-01-1T00:00:00Z".parse().unwrap();

        let state = builder
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => v.landing.landing_timestamp = month1,
                1 => v.landing.landing_timestamp = month2,
                _ => (),
            })
            .build()
            .await;

        let params = LandingsParams {
            months: Some(vec![DateTimeUtc(month1), DateTimeUtc(month2)]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
        assert_eq!(landings, state.landings[..2]);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_in_catch_location() {
    test(|helper, builder| async move {
        let state = builder
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.landing.catch_location.main_area_code = Some(9);
                    v.landing.catch_location.location_code = Some(5);
                }
                1 => {
                    v.landing.catch_location.main_area_code = Some(9);
                    v.landing.catch_location.location_code = Some(4);
                }
                _ => (),
            })
            .build()
            .await;

        let params = LandingsParams {
            catch_locations: Some(vec![
                "09-05".try_into().unwrap(),
                "09-04".try_into().unwrap(),
            ]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);

        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
        assert_eq!(landings, state.landings[..2]);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_gear_group_ids() {
    test(|helper, builder| async move {
        let state = builder
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => v.landing.gear.group = GearGroup::Not,
                1 => v.landing.gear.group = GearGroup::BurOgRuser,
                _ => (),
            })
            .build()
            .await;

        let params = LandingsParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::BurOgRuser),
            ]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
        assert_eq!(landings, state.landings[..2]);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_species_group_ids() {
    test(|helper, builder| async move {
        let state = builder
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => v.landing.product.species.group_code = SpeciesGroup::Blaakveite,
                1 => v.landing.product.species.group_code = SpeciesGroup::Uer,
                _ => (),
            })
            .build()
            .await;

        let params = LandingsParams {
            species_group_ids: Some(vec![
                SpeciesGroupId(SpeciesGroup::Blaakveite),
                SpeciesGroupId(SpeciesGroup::Uer),
            ]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
        assert_eq!(landings, state.landings[..2]);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_vessel_length_ranges() {
    test(|helper, builder| async move {
        let state = builder
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => v.landing.vessel.length = Some(9.),
                1 => v.landing.vessel.length = Some(12.),
                _ => (),
            })
            .build()
            .await;

        let params = LandingsParams {
            vessel_length_ranges: Some(vec!["(,10)".parse().unwrap(), "[10,15)".parse().unwrap()]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 2);
        assert_eq!(landings, state.landings[..2]);
    })
    .await;
}

#[tokio::test]
async fn test_landings_returns_landings_with_fiskeridir_vessel_ids() {
    test(|helper, builder| async move {
        let state = builder.landings(2).vessels(2).landings(2).build().await;

        let params = LandingsParams {
            fiskeridir_vessel_ids: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
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
    test(|helper, builder| async move {
        let state = builder.landings(4).build().await;

        let params = LandingsParams {
            sorting: Some(LandingsSorting::LandingTimestamp),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 4);
        assert_eq!(landings, state.landings);
    })
    .await;
}

#[tokio::test]
async fn test_landings_sorts_by_weight() {
    test(|helper, builder| async move {
        let state = builder
            .landings(4)
            .modify_idx(|i, v| {
                v.landing.product.living_weight = Some(i as f64);
            })
            .build()
            .await;
        let params = LandingsParams {
            sorting: Some(LandingsSorting::LivingWeight),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let response = helper.app.get_landings(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 4);
        assert_eq!(landings, state.landings);
    })
    .await;
}

#[tokio::test]
async fn test_landing_deletion_only_deletes_removed_landings() {
    test(|helper, builder| async move {
        let vessel_id = FiskeridirVesselId(1);
        let landing = fiskeridir_rs::Landing::test_default(1, Some(vessel_id.0));
        let landing2 = fiskeridir_rs::Landing::test_default(2, Some(vessel_id.0));
        let landing3 = fiskeridir_rs::Landing::test_default(3, Some(vessel_id.0));
        helper
            .db
            .add_landings(vec![landing.clone(), landing2.clone(), landing3.clone()])
            .await;

        let landings = helper.db.landing_ids_of_vessel(vessel_id).await;
        assert_eq!(3, landings.len());
        assert_eq!(landings[0], landing.id);
        assert_eq!(landings[1], landing2.id);
        assert_eq!(landings[2], landing3.id);

        helper
            .db
            .add_landings(vec![landing.clone(), landing3.clone()])
            .await;

        let landings = helper.db.landing_ids_of_vessel(vessel_id).await;
        assert_eq!(2, landings.len());
        assert_eq!(landings[0], landing.id);
        assert_eq!(landings[1], landing3.id);

        builder.build().await;
    })
    .await;
}
