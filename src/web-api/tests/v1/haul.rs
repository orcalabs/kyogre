use super::helper::test_with_cache;
use chrono::{DateTime, Utc};
use engine::*;
use fiskeridir_rs::{ErsDca, GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, HaulsSorting, Ordering};
use web_api::routes::v1::haul::HaulsParams;

#[tokio::test]
async fn test_hauls_returns_all_hauls() {
    test_with_cache(|helper, builder| async move {
        let state = builder.hauls(3).build().await;

        helper.refresh_cache().await;

        let hauls = helper.app.get_hauls(Default::default()).await.unwrap();

        assert_eq!(hauls.len(), 3);
        assert_eq!(hauls, state.hauls)
    })
    .await;
}

#[tokio::test]
async fn test_hauls_sorts_by_weight() {
    test_with_cache(|helper, builder| async move {
        let mut state = builder
            .hauls(4)
            .modify_idx(|i, v| v.dca.catch.species.living_weight = Some(i as u32))
            .build()
            .await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            sorting: Some(HaulsSorting::Weight),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        state.hauls.sort_by_key(|v| v.total_living_weight());

        assert_eq!(hauls.len(), 4);
        assert_eq!(hauls, state.hauls);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_in_specified_months() {
    test_with_cache(|helper, builder| async move {
        let month1: DateTime<Utc> = "2000-06-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2001-01-1T00:00:00Z".parse().unwrap();

        let state = builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.set_start_timestamp(month1);
                    v.dca.set_stop_timestamp(month1);
                }
                1 => {
                    v.dca.set_start_timestamp(month2);
                    v.dca.set_stop_timestamp(month2);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            months: Some(vec![month1, month2]),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 2);
        assert_eq!(hauls, state.hauls[0..2])
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_in_catch_location() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                }
                1 => {
                    v.dca.start_latitude = Some(56.756293);
                    v.dca.start_longitude = Some(11.514740);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            catch_locations: Some(vec![CatchLocationId::new(9, 5), CatchLocationId::new(9, 4)]),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 2);
        assert_eq!(hauls, state.hauls[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_gear_group_ids() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.gear.gear_group_code = Some(GearGroup::Seine);
                }
                1 => {
                    v.dca.gear.gear_group_code = Some(GearGroup::LobsterTrapAndFykeNets);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            gear_group_ids: Some(vec![GearGroup::Seine, GearGroup::LobsterTrapAndFykeNets]),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 2);
        assert_eq!(hauls, state.hauls[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_species_group_ids() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.catch.species.species_group_code = Some(SpeciesGroup::GreenlandHalibut);
                }
                1 => {
                    v.dca.catch.species.species_group_code = Some(SpeciesGroup::GoldenRedfish);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            species_group_ids: Some(vec![
                SpeciesGroup::GreenlandHalibut,
                SpeciesGroup::GoldenRedfish,
            ]),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 2);
        assert_eq!(hauls, state.hauls[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_vessel_length_groups() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.vessel_info.length = 9.;
                }
                1 => {
                    v.dca.vessel_info.length = 12.;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            vessel_length_groups: Some(vec![
                VesselLengthGroup::UnderEleven,
                VesselLengthGroup::ElevenToFifteen,
            ]),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 2);
        assert_eq!(hauls, state.hauls[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_with_fiskeridir_vessel_ids() {
    test_with_cache(|helper, builder| async move {
        let vessel_id1 = FiskeridirVesselId::test_new(1);
        let vessel_id2 = FiskeridirVesselId::test_new(2);

        let ers1 = ErsDca::test_default(1, Some(vessel_id1));
        let ers2 = ErsDca::test_default(2, Some(vessel_id2));
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        helper.db.add_ers_dca(vec![ers1, ers2, ers3, ers4]).await;

        let state = builder.hauls(2).vessels(2).hauls(2).build().await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            fiskeridir_vessel_ids: Some(vec![vessel_id1, vessel_id2]),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 2);
        assert_eq!(hauls, state.hauls[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_sorts_by_start_timestamp() {
    test_with_cache(|helper, builder| async move {
        let state = builder.hauls(4).build().await;

        helper.refresh_cache().await;

        let params = HaulsParams {
            sorting: Some(HaulsSorting::StartDate),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 4);
        assert_eq!(hauls, state.hauls);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_sorts_by_stop_timestamp() {
    test_with_cache(|helper, builder| async move {
        let state = builder.hauls(4).build().await;
        let params = HaulsParams {
            sorting: Some(HaulsSorting::StopDate),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.refresh_cache().await;

        let hauls = helper.app.get_hauls(params).await.unwrap();

        assert_eq!(hauls.len(), 4);
        assert_eq!(hauls, state.hauls);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_species_fiskeridir_defaults_to_zero() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .hauls(1)
            .modify(|v| v.dca.catch.species.species_fdir_code = None)
            .build()
            .await;

        helper.refresh_cache().await;

        let hauls = helper.app.get_hauls(Default::default()).await.unwrap();

        assert_eq!(hauls.len(), 1);
        assert_eq!(hauls, state.hauls);
        assert_eq!(hauls[0].catches[0].species_fiskeridir_id, 0);
    })
    .await;
}
