use super::helper::test;
use actix_web::http::StatusCode;
use chrono::Duration;
use fiskeridir_rs::{GearGroup, LandingId, SpeciesGroup};
use kyogre_core::levels::*;
use web_api::routes::v1::vessel::Vessel;

#[tokio::test]
async fn test_vessels_returns_merged_data_from_fiskeridir_and_ais() {
    test(|helper, builder| async move {
        let mut state = builder.vessels(1).build().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();

        assert_eq!(body[0].fiskeridir, state.vessels[0].fiskeridir);
        assert_eq!(
            state.vessels[0].ais.take().unwrap(),
            body[0].ais.take().unwrap()
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_contains_weight_per_hour_benchmark() {
    test(|helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .trips(1)
            .landings(1)
            .modify(|v| {
                v.landing.product.living_weight = Some(1000.0);
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = body.pop().unwrap();

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 500.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_is_correct_over_multiple_trips() {
    test(|helper, builder| async move {
        builder
            .trip_duration(Duration::hours(1))
            .vessels(1)
            .trips(2)
            .landings(1)
            .modify(|v| {
                v.landing.product.living_weight = Some(1000.0);
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = body.pop().unwrap();

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 500.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_includes_landings_not_covered_by_trips() {
    test(|helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .landings(1)
            .modify(|v| {
                v.landing.product.living_weight = Some(1000.0);
            })
            .trips(1)
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = body.pop().unwrap();

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 500.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_excludes_landings_from_other_vessels() {
    test(|helper, builder| async move {
        let state = builder
            .trip_duration(Duration::hours(2))
            .vessels(2)
            .trips(2)
            .landings(2)
            .modify(|v| {
                v.landing.product.living_weight = Some(1000.0);
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 2);

        assert_eq!(state.vessels[0].fish_caught_per_hour.unwrap(), 500.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_is_zero_if_there_are_trips_but_no_landings() {
    test(|helper, builder| async move {
        builder.vessels(1).trips(1).build().await;
        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = &body[0];

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_is_zero_if_there_are_landings_but_no_trips() {
    test(|helper, builder| async move {
        builder.vessels(1).landings(1).build().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = &body[0];

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_zero_gear_groups_with_no_landings() {
    test(|helper, builder| async move {
        builder.vessels(1).build().await;
        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert!(vessel.gear_groups.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_gear_groups_of_landings() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .landings(2)
            .modify_idx(|idx, v| match idx {
                0 => v.landing.gear.group = GearGroup::Not,
                1 => v.landing.gear.group = GearGroup::Garn,
                _ => unreachable!(),
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![GearGroup::Not, GearGroup::Garn], vessel.gear_groups);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_removes_gear_group_when_last_landing_is_replaced_with_new_gear_group() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .landings(1)
            .modify(|v| {
                v.landing.id = LandingId::try_from("1-7-0-0").unwrap();
                v.landing.document_info.version_number = 1;
                v.landing.gear.group = GearGroup::Not;
            })
            .new_cycle()
            .landings(1)
            .modify(|v| {
                v.landing.document_info.version_number = 2;
                v.landing.id = LandingId::try_from("1-7-0-0").unwrap();
                v.landing.gear.group = GearGroup::Garn;
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![GearGroup::Garn], vessel.gear_groups);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_zero_species_groups_with_no_landings() {
    test(|helper, builder| async move {
        builder.vessels(1).build().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert!(vessel.species_groups.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_species_groups_of_landings() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .landings(2)
            .modify_idx(|idx, v| match idx {
                0 => v.landing.product.species.group_code = SpeciesGroup::Torsk,
                1 => v.landing.product.species.group_code = SpeciesGroup::Sei,
                _ => unreachable!(),
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(
            vec![SpeciesGroup::Torsk, SpeciesGroup::Sei],
            vessel.species_groups
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_removes_species_group_when_last_landing_is_replaced_with_new_species_group() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .landings(1)
            .modify(|v| {
                v.landing.id = LandingId::try_from("1-7-0-0").unwrap();
                v.landing.document_info.version_number = 1;
                v.landing.product.species.group_code = SpeciesGroup::Torsk;
            })
            .new_cycle()
            .landings(1)
            .modify(|v| {
                v.landing.document_info.version_number = 2;
                v.landing.id = LandingId::try_from("1-7-0-0").unwrap();
                v.landing.product.species.group_code = SpeciesGroup::Sei;
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![SpeciesGroup::Sei], vessel.species_groups);
    })
    .await;
}
