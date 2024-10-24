use engine::*;
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use kyogre_core::{
    ActiveVesselConflict, FiskeridirVesselId, Mmsi, TestHelperOutbound, VesselSource,
};

use super::helper::test;

#[tokio::test]
async fn test_vessels_returns_merged_data_from_fiskeridir_and_ais() {
    test(|helper, builder| async move {
        let mut state = builder.vessels(1).build().await;

        let mut vessels = helper.app.get_vessels().await.unwrap();

        assert_eq!(vessels[0].fiskeridir, state.vessels[0].fiskeridir);
        assert_eq!(
            state.vessels[0].ais.take().unwrap(),
            vessels[0].ais.take().unwrap()
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_zero_gear_groups_with_no_landings() {
    test(|helper, builder| async move {
        builder.vessels(1).build().await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert!(vessels[0].gear_groups.is_empty());
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

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(
            vec![GearGroup::Seine, GearGroup::Net],
            vessels[0].gear_groups
        );
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
                v.landing.id = "1-7-0-0".parse().unwrap();
                v.landing.document_info.version_number = 1;
                v.landing.gear.group = GearGroup::Seine;
            })
            .new_cycle()
            .landings(1)
            .modify(|v| {
                v.landing.document_info.version_number = 2;
                v.landing.id = "1-7-0-0".parse().unwrap();
                v.landing.gear.group = GearGroup::Net;
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vec![GearGroup::Net], vessels[0].gear_groups);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_zero_species_groups_with_no_landings() {
    test(|helper, builder| async move {
        builder.vessels(1).build().await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert!(vessels[0].species_groups.is_empty());
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

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(
            vec![SpeciesGroup::AtlanticCod, SpeciesGroup::Saithe],
            vessels[0].species_groups
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
                v.landing.id = "1-7-0-0".parse().unwrap();
                v.landing.document_info.version_number = 1;
                v.landing.product.species.group_code = SpeciesGroup::AtlanticCod;
            })
            .new_cycle()
            .landings(1)
            .modify(|v| {
                v.landing.document_info.version_number = 2;
                v.landing.id = "1-7-0-0".parse().unwrap();
                v.landing.product.species.group_code = SpeciesGroup::Saithe;
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vec![SpeciesGroup::Saithe], vessels[0].species_groups);
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_vessels_that_only_exists_in_landings_without_call_sign() {
    test(|helper, builder| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        builder
            .landings(1)
            .modify(|l| {
                l.landing.vessel.id = Some(vessel_id);
                l.landing.vessel.call_sign = None;
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vessels[0].fiskeridir.id, vessel_id);
        assert!(vessels[0].fiskeridir.call_sign.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_vessels_that_only_exists_in_landings_with_call_sign() {
    test(|helper, builder| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        builder
            .landings(1)
            .modify(|l| {
                l.landing.vessel.id = Some(vessel_id);
                l.landing.vessel.call_sign = Some("test".parse().unwrap());
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vessels[0].fiskeridir.id, vessel_id);
        assert!(vessels[0].ais.is_none());
        assert_eq!(vessels[0].fiskeridir_call_sign().unwrap(), "test");
    })
    .await;
}

#[tokio::test]
async fn test_vessels_does_not_return_vessel_with_an_active_conflict() {
    test(|helper, builder| async move {
        builder
            .vessels(2)
            .modify_idx(|i, v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(i as i64);
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        let conflicts = helper.adapter().active_vessel_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0],
            ActiveVesselConflict {
                vessel_ids: vec![
                    Some(FiskeridirVesselId::test_new(0)),
                    Some(FiskeridirVesselId::test_new(1))
                ],
                call_sign: "test".parse().unwrap(),
                mmsis: vec![Some(Mmsi::test_new(1)), Some(Mmsi::test_new(2))],
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
                v.landing.vessel.id = Some(FiskeridirVesselId::test_new(1));
                if i == 0 {
                    v.landing.vessel.call_sign = Some("test".parse().unwrap());
                } else {
                    v.landing.vessel.call_sign = Some("test2".parse().unwrap());
                }
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vessels[0].fiskeridir_call_sign().unwrap(), "test2");
    })
    .await;
}

#[tokio::test]
async fn test_vessels_does_not_return_most_used_call_sign_of_vessel_that_exists_in_register() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = Some("cs".parse().unwrap());
            })
            .landings(3)
            .modify(|v| {
                v.landing.vessel.call_sign = Some("test".parse().unwrap());
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vessels[0].fiskeridir_call_sign().unwrap(), "cs");
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_both_vessels_after_conflict_have_been_resolved_but_loser_without_call_sign_and_ais(
) {
    test(|helper, builder| async move {
        let vessel_id1 = FiskeridirVesselId::test_new(1);
        let vessel_id2 = FiskeridirVesselId::test_new(2);

        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id1;
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(1);
            })
            .conflict_winner()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id2;
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(2);
            })
            .conflict_loser()
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let winner = vessels
            .iter()
            .find(|v| v.fiskeridir.id == vessel_id1)
            .unwrap();
        let loser = vessels
            .iter()
            .find(|v| v.fiskeridir.id == vessel_id2)
            .unwrap();

        assert_eq!(winner.mmsi().unwrap().into_inner(), 1);
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
                v.fiskeridir.id = FiskeridirVesselId::test_new(i as i64);
                v.fiskeridir.radio_call_sign = Some("test".to_string().parse().unwrap());
                v.ais.call_sign = Some("test".to_string().parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(1);
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert!(vessels.is_empty());

        let conflicts = helper.adapter().active_vessel_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0],
            ActiveVesselConflict {
                vessel_ids: vec![
                    Some(FiskeridirVesselId::test_new(0)),
                    Some(FiskeridirVesselId::test_new(1))
                ],
                call_sign: "test".parse().unwrap(),
                mmsis: vec![Some(Mmsi::test_new(1))],
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
                v.fiskeridir.id = FiskeridirVesselId::test_new(i as i64);
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(1);
            })
            .conflict_winner()
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let vessel = vessels
            .iter()
            .find(|v| v.fiskeridir.id.into_inner() == 0)
            .unwrap();
        let vessel2 = vessels
            .iter()
            .find(|v| v.fiskeridir.id.into_inner() == 1)
            .unwrap();

        assert_eq!(vessel.mmsi().unwrap().into_inner(), 1);
        assert_eq!(vessel.ais_call_sign().unwrap(), "test");
        assert_eq!(vessel2.mmsi().unwrap().into_inner(), 1);
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
                v.vessel.mmsi = Mmsi::test_new(2);
                v.vessel.call_sign = Some("test".parse().unwrap());
            })
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(1);
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert!(vessels.is_empty());

        let conflicts = helper.adapter().active_vessel_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0],
            ActiveVesselConflict {
                vessel_ids: vec![Some(FiskeridirVesselId::test_new(1))],
                call_sign: "test".parse().unwrap(),
                mmsis: vec![Some(Mmsi::test_new(1)), Some(Mmsi::test_new(2))],
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
                v.vessel.mmsi = Mmsi::test_new(2);
                v.vessel.call_sign = Some("test".parse().unwrap());
            })
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(1);
            })
            .conflict_winner()
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 1);
        assert_eq!(vessels[0].mmsi().unwrap().into_inner(), 1);
        assert_eq!(vessels[0].ais_call_sign().unwrap(), "test");
        assert_eq!(vessels[0].fiskeridir_call_sign().unwrap(), "test");
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
                    v.fiskeridir.radio_call_sign = Some("0".parse().unwrap());
                } else {
                    v.fiskeridir.radio_call_sign = Some("00000000".parse().unwrap());
                }
            })
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 2);

        assert!(vessels[0].fiskeridir_call_sign().is_none());
        assert!(vessels[1].fiskeridir_call_sign().is_none());
        assert!(helper.adapter().active_vessel_conflicts().await.is_empty());
    })
    .await;
}
