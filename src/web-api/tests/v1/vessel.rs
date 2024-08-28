use super::helper::test;
use chrono::Duration;
use engine::*;
use fiskeridir_rs::{GearGroup, LandingId, SpeciesGroup};
use kyogre_core::{
    ActiveVesselConflict, FiskeridirVesselId, Mmsi, TestHelperOutbound, VesselSource,
};
use reqwest::StatusCode;
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
                0 => v.landing.gear.group = GearGroup::Seine,
                1 => v.landing.gear.group = GearGroup::Net,
                _ => unreachable!(),
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![GearGroup::Seine, GearGroup::Net], vessel.gear_groups);
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
                v.landing.gear.group = GearGroup::Seine;
            })
            .new_cycle()
            .landings(1)
            .modify(|v| {
                v.landing.document_info.version_number = 2;
                v.landing.id = LandingId::try_from("1-7-0-0").unwrap();
                v.landing.gear.group = GearGroup::Net;
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![GearGroup::Net], vessel.gear_groups);
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
                0 => v.landing.product.species.group_code = SpeciesGroup::AtlanticCod,
                1 => v.landing.product.species.group_code = SpeciesGroup::Saithe,
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
            vec![SpeciesGroup::AtlanticCod, SpeciesGroup::Saithe],
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
                v.landing.product.species.group_code = SpeciesGroup::AtlanticCod;
            })
            .new_cycle()
            .landings(1)
            .modify(|v| {
                v.landing.document_info.version_number = 2;
                v.landing.id = LandingId::try_from("1-7-0-0").unwrap();
                v.landing.product.species.group_code = SpeciesGroup::Saithe;
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![SpeciesGroup::Saithe], vessel.species_groups);
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_vessels_that_only_exists_in_landings_without_call_sign() {
    test(|helper, builder| async move {
        builder
            .landings(1)
            .modify(|l| {
                l.landing.vessel.id = Some(1);
                l.landing.vessel.call_sign = None;
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 1);

        let vessel = &vessels[0];
        assert_eq!(vessel.fiskeridir.id.0, 1);
        assert!(vessel.fiskeridir.call_sign.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_vessels_that_only_exists_in_landings_with_call_sign() {
    test(|helper, builder| async move {
        builder
            .landings(1)
            .modify(|l| {
                l.landing.vessel.id = Some(1);
                l.landing.vessel.call_sign = Some("test".try_into().unwrap());
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 1);

        let vessel = &vessels[0];
        assert_eq!(vessel.fiskeridir.id.0, 1);
        assert!(vessel.ais.is_none());
        assert_eq!(vessel.fiskeridir_call_sign().unwrap(), "test");
    })
    .await;
}

#[tokio::test]
async fn test_vessels_does_not_return_vessel_with_an_active_conflict() {
    test(|helper, builder| async move {
        builder
            .vessels(2)
            .modify_idx(|i, v| {
                v.fiskeridir.id = i as i64;
                v.fiskeridir.radio_call_sign = Some("test".try_into().unwrap());
                v.ais.call_sign = Some("test".try_into().unwrap());
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();

        let conflicts = helper.adapter().active_vessel_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0],
            ActiveVesselConflict {
                vessel_ids: vec![Some(FiskeridirVesselId(0)), Some(FiskeridirVesselId(1))],
                call_sign: "test".try_into().unwrap(),
                mmsis: vec![Some(Mmsi(1)), Some(Mmsi(2))],
                sources: vec![Some(VesselSource::FiskeridirVesselRegister)],
            }
        );
        assert!(vessels.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_most_used_call_sign_of_vessel_that_only_exists_in_landings() {
    test(|helper, builder| async move {
        builder
            .landings(3)
            .modify_idx(|i, v| {
                v.landing.vessel.id = Some(1);
                if i == 0 {
                    v.landing.vessel.call_sign = Some("test".try_into().unwrap());
                } else {
                    v.landing.vessel.call_sign = Some("test2".try_into().unwrap());
                }
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 1);

        let vessel = &vessels[0];
        assert_eq!(vessel.fiskeridir_call_sign().unwrap(), "test2");
    })
    .await;
}

#[tokio::test]
async fn test_vessels_does_not_return_most_used_call_sign_of_vessel_that_exists_in_register() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = Some("cs".try_into().unwrap());
            })
            .landings(3)
            .modify(|v| {
                v.landing.vessel.call_sign = Some("test".try_into().unwrap());
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 1);

        let vessel = &vessels[0];
        assert_eq!(vessel.fiskeridir_call_sign().unwrap(), "cs");
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_both_vessels_after_conflict_have_been_resolved_but_loser_without_call_sign_and_ais(
) {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = 1;
                v.fiskeridir.radio_call_sign = Some("test".try_into().unwrap());
                v.ais.call_sign = Some("test".try_into().unwrap());
                v.ais.mmsi = Mmsi(1);
            })
            .conflict_winner()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = 2;
                v.fiskeridir.radio_call_sign = Some("test".try_into().unwrap());
                v.ais.call_sign = Some("test".try_into().unwrap());
                v.ais.mmsi = Mmsi(2);
            })
            .conflict_loser()
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let winner = vessels.iter().find(|v| v.fiskeridir.id.0 == 1).unwrap();
        let loser = vessels.iter().find(|v| v.fiskeridir.id.0 == 2).unwrap();

        assert_eq!(winner.mmsi().unwrap().0, 1);
        assert_eq!(winner.ais_call_sign().unwrap(), "test");
        assert!(loser.ais.is_none());
        assert!(loser.fiskeridir_call_sign().is_none());

        assert!(helper.adapter().active_vessel_conflicts().await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessels_does_not_return_vessels_with_an_active_mmsi_conflict() {
    test(|helper, builder| async move {
        builder
            .vessels(2)
            .modify_idx(|i, v| {
                v.fiskeridir.id = i as i64;
                v.fiskeridir.radio_call_sign = Some("test".to_string().try_into().unwrap());
                v.ais.call_sign = Some("test".to_string().try_into().unwrap());
                v.ais.mmsi = Mmsi(1);
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert!(vessels.is_empty());

        let conflicts = helper.adapter().active_vessel_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0],
            ActiveVesselConflict {
                vessel_ids: vec![Some(FiskeridirVesselId(0)), Some(FiskeridirVesselId(1))],
                call_sign: "test".try_into().unwrap(),
                mmsis: vec![Some(Mmsi(1))],
                sources: vec![Some(VesselSource::FiskeridirVesselRegister)],
            },
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_vessels_conflicts_that_have_been_annotated_as_the_same_vessel() {
    test(|helper, builder| async move {
        builder
            .vessels(2)
            .modify_idx(|i, v| {
                v.fiskeridir.id = i as i64;
                v.fiskeridir.radio_call_sign = Some("test".try_into().unwrap());
                v.ais.call_sign = Some("test".try_into().unwrap());
                v.ais.mmsi = Mmsi(1);
            })
            .conflict_winner()
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let vessel = vessels.iter().find(|v| v.fiskeridir.id.0 == 0).unwrap();
        let vessel2 = vessels.iter().find(|v| v.fiskeridir.id.0 == 1).unwrap();

        assert_eq!(vessel.mmsi().unwrap().0, 1);
        assert_eq!(vessel.ais_call_sign().unwrap(), "test");
        assert_eq!(vessel2.mmsi().unwrap().0, 1);
        assert_eq!(vessel2.ais_call_sign().unwrap(), "test");

        assert!(helper.adapter().active_vessel_conflicts().await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessels_does_not_return_an_active_mmsi_conflict() {
    test(|helper, builder| async move {
        builder
            .ais_vessels(1)
            .modify(|v| {
                v.vessel.mmsi = Mmsi(2);
                v.vessel.call_sign = Some("test".try_into().unwrap());
            })
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = Some("test".try_into().unwrap());
                v.ais.call_sign = Some("test".try_into().unwrap());
                v.ais.mmsi = Mmsi(1);
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 0);

        let conflicts = helper.adapter().active_vessel_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0],
            ActiveVesselConflict {
                vessel_ids: vec![Some(FiskeridirVesselId(1))],
                call_sign: "test".try_into().unwrap(),
                mmsis: vec![Some(Mmsi(1)), Some(Mmsi(2))],
                sources: vec![Some(VesselSource::FiskeridirVesselRegister)],
            },
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessels_only_returns_winner_of_resolved_mmsi_conflict() {
    test(|helper, builder| async move {
        builder
            .ais_vessels(1)
            .modify(|v| {
                v.vessel.mmsi = Mmsi(2);
                v.vessel.call_sign = Some("test".try_into().unwrap());
            })
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = Some("test".try_into().unwrap());
                v.ais.call_sign = Some("test".try_into().unwrap());
                v.ais.mmsi = Mmsi(1);
            })
            .conflict_winner()
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 1);

        let vessel = &vessels[0];

        assert_eq!(vessel.mmsi().unwrap().0, 1);
        assert_eq!(vessel.ais_call_sign().unwrap(), "test");
        assert_eq!(vessel.fiskeridir_call_sign().unwrap(), "test");
        assert!(helper.adapter().active_vessel_conflicts().await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessels_with_ignored_call_signs_have_no_call_sign() {
    test(|helper, builder| async move {
        builder
            .vessels(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.fiskeridir.radio_call_sign = Some("0".try_into().unwrap());
                } else {
                    v.fiskeridir.radio_call_sign = Some("00000000".try_into().unwrap());
                }
            })
            .build()
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let vessel = &vessels[0];
        let vessel2 = &vessels[1];

        assert!(vessel.fiskeridir_call_sign().is_none());
        assert!(vessel2.fiskeridir_call_sign().is_none());
        assert!(helper.adapter().active_vessel_conflicts().await.is_empty());
    })
    .await;
}
