use super::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::SpeciesGroup;
use fiskeridir_rs::{CallSign, GearGroup};
use float_cmp::approx_eq;
use http_client::StatusCode;
use kyogre_core::ScraperInboundPort;
use kyogre_core::{
    ActiveVesselConflict, FiskeridirVesselId, Mmsi, ProcessingStatus, TestHelperOutbound,
    UpdateVessel, VesselSource,
};
use kyogre_core::{DEFAULT_LIVE_FUEL_THRESHOLD, TEST_SIGNED_IN_VESSEL_CALLSIGN};
use kyogre_core::{FuelEstimation, NaiveDateRange};
use web_api::routes::v1::vessel::{FuelParams, LiveFuelParams};

pub mod benchmarks;
pub mod fuel;

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
async fn test_vessels_returns_vessels_conflicts_that_have_been_annotated_as_the_same_vessel() {
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
            .active_vessel()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id2;
                v.fiskeridir.radio_call_sign = Some("test".parse().unwrap());
                v.ais.call_sign = Some("test".parse().unwrap());
                v.ais.mmsi = Mmsi::test_new(1);
            })
            .historic_vessel()
            .build()
            .await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let active = vessels
            .iter()
            .find(|v| v.fiskeridir.id == vessel_id1)
            .unwrap();
        let historic = vessels
            .iter()
            .find(|v| v.fiskeridir.id == vessel_id2)
            .unwrap();

        assert_eq!(active.mmsi().unwrap().into_inner(), 1);
        assert_eq!(active.ais_call_sign().unwrap(), "test");
        assert_eq!(historic.mmsi().unwrap().into_inner(), 1);
        assert_eq!(historic.ais_call_sign().unwrap(), "test");

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
            .active_vessel()
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

#[tokio::test]
async fn test_update_vessel_succeeds() {
    test(|mut helper, builder| async move {
        let state = builder.vessels(1).set_logged_in().build().await;

        let update = UpdateVessel {
            engine_power: Some(2000),
            engine_building_year: Some(1233231),
            auxiliary_engine_power: Some(100),
            boiler_engine_power: Some(50),
            auxiliary_engine_building_year: Some(1233231),
            boiler_engine_building_year: Some(1233231),
            degree_of_electrification: Some(0.5),
            service_speed: Some(15.0),
        };
        helper.app.login_user();
        let new_vessel = helper.app.update_vessel(&update).await.unwrap();
        let vessels = helper
            .app
            .get_vessels()
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.fiskeridir.id == state.vessels[0].fiskeridir.id)
            .unwrap();

        assert_eq!(update, new_vessel);
        assert_eq!(vessels, new_vessel);
    })
    .await;
}

#[tokio::test]
async fn test_update_vessel_resets_benchmarks() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .ais_vms_positions(3)
            .hauls(3)
            .build()
            .await;

        let update = UpdateVessel {
            engine_power: Some(2000),
            engine_building_year: Some(1233231),
            auxiliary_engine_power: Some(100),
            boiler_engine_power: Some(50),
            auxiliary_engine_building_year: Some(1233231),
            boiler_engine_building_year: Some(1233231),
            degree_of_electrification: Some(0.5),
            service_speed: Some(15.0),
        };
        helper.app.login_user();
        helper.app.update_vessel(&update).await.unwrap();

        assert!(
            helper
                .db
                .db
                .trips_with_benchmark_status(ProcessingStatus::Unprocessed)
                .await
                > 0
        );
    })
    .await;
}
#[tokio::test]
async fn test_update_vessel_resets_fuel_estimation() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .ais_vms_positions(3)
            .hauls(3)
            .build()
            .await;

        let update = UpdateVessel {
            engine_power: Some(2000),
            engine_building_year: Some(1233231),
            auxiliary_engine_power: Some(100),
            boiler_engine_power: Some(50),
            auxiliary_engine_building_year: Some(1233231),
            boiler_engine_building_year: Some(1233231),
            degree_of_electrification: Some(0.5),
            service_speed: Some(15.0),
        };
        helper.app.login_user();
        helper.app.update_vessel(&update).await.unwrap();

        assert!(
            helper
                .db
                .db
                .fuel_estimates_with_status(ProcessingStatus::Unprocessed)
                .await
                > 0
        );
        assert_eq!(
            helper
                .db
                .db
                .fuel_estimates_with_status(ProcessingStatus::Successful)
                .await,
            0
        );
    })
    .await;
}

#[tokio::test]
async fn test_cant_use_fuel_endpoint_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper
            .app
            .get_vessel_fuel(Default::default())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_fuel_is_estimated() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .ais_vms_positions(10)
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(
                    state.ais_vms_positions[0].timestamp.naive_utc().date(),
                    state.ais_vms_positions[9].timestamp.naive_utc().date(),
                ),
            })
            .await
            .unwrap();

        assert!(fuel > 0.0)
    })
    .await;
}

#[tokio::test]
async fn test_fuel_is_equal_to_trip_fuel_estimation() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .ais_vms_positions(10)
            .build()
            .await;

        helper.app.login_user();

        let trips = helper.app.get_trips(Default::default()).await.unwrap();
        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(
                    state.ais_vms_positions[0].timestamp.naive_utc().date(),
                    state.ais_vms_positions[9].timestamp.naive_utc().date(),
                ),
            })
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert!(approx_eq!(f64, fuel, trips[0].fuel_consumption.unwrap()))
    })
    .await;
}

#[tokio::test]
async fn test_fuel_returns_zero_if_no_estimate_exists() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(Default::default())
            .await
            .unwrap();
        assert_eq!(fuel as i32, 0);
    })
    .await;
}

#[tokio::test]
async fn test_fuel_is_not_recalculated_with_new_hauls_with_passive_gear_types() {
    test(|mut helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap();
        let end = start + Duration::hours(10);
        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(TEST_SIGNED_IN_VESSEL_CALLSIGN).unwrap());
            })
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .vms_positions(10)
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(
                    state.ais_vms_positions[0].timestamp.naive_utc().date(),
                    state.ais_vms_positions[9].timestamp.naive_utc().date(),
                ),
            })
            .await
            .unwrap();

        helper
            .builder()
            .await
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(TEST_SIGNED_IN_VESSEL_CALLSIGN).unwrap());
            })
            .hauls(1)
            .modify(|h| {
                h.dca.gear.gear_group_code = Some(GearGroup::Net);
                h.dca.set_start_timestamp(start + Duration::seconds(1));
                h.dca.set_stop_timestamp(end - Duration::seconds(1));
            })
            .build()
            .await;

        let fuel2 = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        assert!(approx_eq!(f64, fuel, fuel2))
    })
    .await;
}
#[tokio::test]
async fn test_fuel_is_recalculated_with_new_hauls() {
    test(|mut helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap();
        let end = start + Duration::hours(10);
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(TEST_SIGNED_IN_VESSEL_CALLSIGN).unwrap());
            })
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .vms_positions(10)
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        helper
            .builder()
            .await
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(TEST_SIGNED_IN_VESSEL_CALLSIGN).unwrap());
            })
            .hauls(1)
            .modify(|h| {
                h.dca.gear.gear_group_code = Some(GearGroup::Trawl);
                h.dca.set_start_timestamp(start + Duration::seconds(1));
                h.dca.set_stop_timestamp(end - Duration::seconds(1));
            })
            .build()
            .await;

        let fuel2 = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        assert!(
            !approx_eq!(f64, fuel, fuel2),
            "before: {fuel}, after: {fuel2}"
        );
        assert!(fuel2 > fuel);
    })
    .await;
}

#[tokio::test]
async fn test_fuel_is_recalculated_with_new_vms_data() {
    test(|mut helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap();
        let end = start + Duration::days(10);
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(TEST_SIGNED_IN_VESSEL_CALLSIGN).unwrap());
            })
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .vms_positions(10)
            .modify_idx(|i, p| {
                p.position.timestamp = end - Duration::hours(i as i64);
            })
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        helper
            .builder()
            .await
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(TEST_SIGNED_IN_VESSEL_CALLSIGN).unwrap());
            })
            .vms_positions(10)
            .modify_idx(|i, p| {
                p.position.timestamp = start + Duration::hours(i as i64);
            })
            .build()
            .await;

        let fuel2 = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        assert!(!approx_eq!(f64, fuel, fuel2))
    })
    .await;
}
#[tokio::test]
async fn test_live_fuel_returns_all_fuel_within_default_threshold() {
    test(|mut helper, builder| async move {
        let start = Utc::now() - DEFAULT_LIVE_FUEL_THRESHOLD - Duration::hours(1);
        builder
            .vessels(1)
            .set_engine_building_year()
            .set_logged_in()
            .ais_positions(20)
            .modify_idx(|i, p| {
                p.position.msgtime = start + Duration::minutes((i * 20) as i64);
            })
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_live_vessel_fuel(LiveFuelParams::default())
            .await
            .unwrap();

        assert_eq!(fuel.entries.len(), 5);
        assert!(!approx_eq!(f64, fuel.total_fuel_liter, 0.0));
        assert!(approx_eq!(
            f64,
            fuel.entries.iter().map(|e| e.fuel_liter).sum::<f64>(),
            fuel.total_fuel_liter
        ));
    })
    .await;
}

#[tokio::test]
async fn test_vessels_returns_correct_current_trip() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(3)
            .dep(2)
            .modify_idx(|i, d| {
                if i == 0 {
                    d.dep.target_species_fdir_code = Some(100);
                } else {
                    d.dep.target_species_fdir_code = Some(101);
                }
            })
            .build()
            .await;

        let mut vessels = helper.app.get_vessels().await.unwrap();
        vessels.sort_by_key(|v| v.fiskeridir.id);

        let trip = vessels[0].current_trip.as_ref().unwrap();
        let trip2 = vessels[1].current_trip.as_ref().unwrap();
        assert_eq!(trip.departure, state.dep[0].timestamp);
        assert_eq!(trip.target_species_fiskeridir_id, Some(100));

        assert_eq!(trip2.departure, state.dep[1].timestamp);
        assert_eq!(trip2.target_species_fiskeridir_id, Some(101));

        assert!(vessels[2].current_trip.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_update_vessel_with_engine_info_recomputes_fuel_for_trips_and_day_estimates() {
    test(|mut helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap();
        let end = start + Duration::days(10);
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .vms_positions(10)
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        let trip_fuel = helper
            .app
            .get_trips(Default::default())
            .await
            .unwrap()
            .pop()
            .unwrap()
            .fuel_consumption
            .unwrap();

        helper
            .app
            .update_vessel(&UpdateVessel {
                engine_power: Some(2000),
                engine_building_year: Some(2000),
                auxiliary_engine_power: Some(2000),
                auxiliary_engine_building_year: Some(2000),
                boiler_engine_power: Some(2000),
                boiler_engine_building_year: Some(2000),
                degree_of_electrification: Some(0.5),
                service_speed: Some(15.0),
            })
            .await
            .unwrap();

        helper.builder().await.build().await;

        let trip_fuel2 = helper
            .app
            .get_trips(Default::default())
            .await
            .unwrap()
            .pop()
            .unwrap()
            .fuel_consumption
            .unwrap();
        let fuel2 = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        assert!(fuel2 > fuel);
        assert!(trip_fuel2 > trip_fuel);
    })
    .await;
}

#[tokio::test]
async fn test_update_vessel_with_engine_info_stops_fuel_estimation_from_comitting_with_stale_info()
{
    test(|mut helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap();
        let end = start + Duration::days(10);
        let processors = builder.processors.clone();

        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(10)
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        let vessels = helper.adapter().vessels_with_trips(1).await.unwrap();
        let cs = vessels[0].fiskeridir.call_sign.clone().unwrap();

        helper
            .app
            .update_vessel(&UpdateVessel {
                engine_power: Some(2000),
                engine_building_year: Some(2000),
                auxiliary_engine_power: Some(2000),
                auxiliary_engine_building_year: Some(2000),
                boiler_engine_power: Some(2000),
                boiler_engine_building_year: Some(2000),
                degree_of_electrification: Some(0.5),
                service_speed: Some(15.0),
            })
            .await
            .unwrap();

        helper
            .adapter()
            .add_vms(vec![
                fiskeridir_rs::Vms::test_default(100, cs.clone(), end + Duration::seconds(1)),
                fiskeridir_rs::Vms::test_default(101, cs, end + Duration::seconds(2)),
            ])
            .await
            .unwrap();

        processors
            .estimator
            .run_single(Some(vessels))
            .await
            .unwrap();

        let fuel2 = helper
            .app
            .get_vessel_fuel(FuelParams {
                range: NaiveDateRange::test_new(start.date_naive(), end.date_naive()),
            })
            .await
            .unwrap();

        assert!(approx_eq!(f64, fuel, fuel2));
    })
    .await;
}
