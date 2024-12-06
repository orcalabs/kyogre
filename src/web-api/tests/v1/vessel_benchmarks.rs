use std::str::FromStr;

use super::helper::test;
use chrono::{Datelike, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::{NonEmptyString, OrgId, RegisterVesselEntityType, RegisterVesselOwner};
use http_client::StatusCode;
use kyogre_core::{Haul, OrgBenchmarks, TripDetailed};
use web_api::routes::v1::{user::User, vessel::OrgBenchmarkParameters};

#[tokio::test]
async fn test_vessel_benchmarks_without_token_returns_not_found() {
    test(|helper, _builder| async move {
        let error = helper.app.get_vessel_benchmarks().await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_cumulative_landings() {
    test(|mut helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(1)
            .set_logged_in()
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(2022, 1, 1, 1, 0, 0).unwrap();
                    v.landing.product.species.fdir_code = 201
                }
                1 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(now.year(), 2, 1, 1, 0, 0).unwrap();
                    v.landing.product.living_weight = Some(200.0);
                    v.landing.product.species.fdir_code = 201
                }
                2 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(now.year(), 3, 1, 1, 0, 0).unwrap();
                    v.landing.product.living_weight = Some(300.0);
                    v.landing.product.species.fdir_code = 201
                }
                3 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(now.year(), 3, 1, 1, 0, 0).unwrap();
                    v.landing.product.living_weight = Some(5000.0);
                    v.landing.product.species.fdir_code = 200
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        helper.app.login_user();

        let benchmarks = helper.app.get_vessel_benchmarks().await.unwrap();
        assert_eq!(benchmarks.cumulative_landings.len(), 3);
        assert_eq!(benchmarks.cumulative_landings[0].species_fiskeridir_id, 201);
        assert_eq!(benchmarks.cumulative_landings[1].species_fiskeridir_id, 200);
        assert_eq!(benchmarks.cumulative_landings[2].species_fiskeridir_id, 201);

        assert_eq!(
            benchmarks.cumulative_landings[0].month,
            chrono::Month::February
        );
        assert_eq!(
            benchmarks.cumulative_landings[1].month,
            chrono::Month::March
        );
        assert_eq!(
            benchmarks.cumulative_landings[2].month,
            chrono::Month::March
        );

        assert_eq!(benchmarks.cumulative_landings[0].weight as i32, 200);
        assert_eq!(benchmarks.cumulative_landings[1].weight as i32, 5000);
        assert_eq!(benchmarks.cumulative_landings[2].weight as i32, 300);

        assert_eq!(
            benchmarks.cumulative_landings[0].cumulative_weight as i32,
            200
        );
        assert_eq!(
            benchmarks.cumulative_landings[1].cumulative_weight as i32,
            5000
        );
        assert_eq!(
            benchmarks.cumulative_landings[2].cumulative_weight as i32,
            500
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_self_benchmarks() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .set_logged_in()
            .trips(2)
            .hauls(4)
            .landings(4)
            .ais_positions(4)
            .build()
            .await;

        helper.app.login_user();

        let benchmarks = helper.app.get_vessel_benchmarks().await.unwrap();

        let fishing_distance = benchmarks.fishing_distance.unwrap();
        let fishing_time = benchmarks.fishing_time.unwrap();
        let trip_time = benchmarks.trip_time.unwrap();
        let landings = benchmarks.landings.unwrap();
        let ers_dca = benchmarks.ers_dca.unwrap();

        // All hauls in test have same duration
        let fishing_time_per_trip = (state.hauls[0].duration().num_minutes() * 2) as f64;
        let trip_time_minutes =
            (state.trips[0].period.end() - state.trips[0].period.start()).num_minutes() as f64;
        // All landings in test have same weight
        let landing_weight_per_trip = state.landings[0].total_living_weight * 2.0;
        // All hauls in test have same weight
        let haul_weight_per_trip = state.hauls[0].total_living_weight() as f64 * 2.0;

        // Fishing time
        assert_eq!(
            fishing_time.recent_trips[0],
            (&state.trips[0], fishing_time_per_trip),
        );
        assert_eq!(
            fishing_time.recent_trips[1],
            (&state.trips[1], fishing_time_per_trip),
        );
        assert_eq!(fishing_time.average_followers, 0.0);
        assert_eq!(fishing_time.average, fishing_time_per_trip);
        // Fishing distance
        assert_eq!(fishing_distance.recent_trips[0], (&state.trips[0], 116.0));
        assert_eq!(fishing_distance.recent_trips[1], (&state.trips[1], 116.0));
        assert_eq!(fishing_distance.average_followers, 0.0);
        assert_eq!(fishing_distance.average as i64, 116);
        // Trip time
        assert_eq!(
            trip_time.recent_trips[0],
            (&state.trips[0], trip_time_minutes),
        );
        assert_eq!(
            trip_time.recent_trips[1],
            (&state.trips[1], trip_time_minutes),
        );
        assert_eq!(trip_time.average_followers, 0.0);
        assert_eq!(trip_time.average, trip_time_minutes);
        // Landings
        assert_eq!(
            landings.recent_trips[0],
            (&state.trips[0], landing_weight_per_trip),
        );
        assert_eq!(
            landings.recent_trips[1],
            (&state.trips[1], landing_weight_per_trip),
        );
        assert_eq!(landings.average_followers, 0.0);
        assert_eq!(landings.average, landing_weight_per_trip);
        // Ers dca
        assert_eq!(
            ers_dca.recent_trips[0],
            (&state.trips[0], haul_weight_per_trip),
        );
        assert_eq!(
            ers_dca.recent_trips[1],
            (&state.trips[1], haul_weight_per_trip),
        );
        assert_eq!(ers_dca.average_followers, 0.0);
        assert_eq!(ers_dca.average, haul_weight_per_trip);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_averages_for_followers() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(3)
            .set_logged_in()
            .trips(3)
            .hauls(6)
            .landings(6)
            .ais_positions(6)
            .build()
            .await;

        helper.app.login_user();
        helper
            .app
            .update_user(User {
                following: vec![
                    state.vessels[1].fiskeridir.id,
                    state.vessels[2].fiskeridir.id,
                ],
            })
            .await
            .unwrap();

        let benchmarks = helper.app.get_vessel_benchmarks().await.unwrap();

        let fishing_distance = benchmarks.fishing_distance.unwrap();
        let fishing_time = benchmarks.fishing_time.unwrap();
        let trip_time = benchmarks.trip_time.unwrap();
        let landings = benchmarks.landings.unwrap();
        let ers_dca = benchmarks.ers_dca.unwrap();

        assert_eq!(
            fishing_time.average_followers as i64,
            state.hauls[0].duration().num_minutes() * 2
        );
        assert_eq!(fishing_distance.average_followers as i64, 116);
        assert_eq!(
            trip_time.average_followers as i64,
            (state.trips[0].period.end() - state.trips[0].period.start()).num_minutes()
        );
        assert_eq!(
            landings.average_followers as i64,
            (state.landings[0].total_living_weight * 2.0) as i64
        );
        assert_eq!(
            ers_dca.average_followers as i32,
            state.hauls[0].total_living_weight() * 2
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_org_benchmarks_without_token_returns_not_found() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).build().await;

        let error = helper
            .app
            .get_org_benchmarks(
                state.vessels[0].fiskeridir.owners[0].id.unwrap(),
                Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_org_benchmarks_returns_not_found_on_org_not_associated_with_vessel() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let org_id2 = OrgId::test_new(2);
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                let org = if i > 0 { org_id } else { org_id2 };
                v.fiskeridir.owners = vec![RegisterVesselOwner {
                    city: None,
                    entity_type: RegisterVesselEntityType::Company,
                    id: Some(org),
                    name: NonEmptyString::from_str("test").unwrap(),
                    postal_code: 9000,
                }];
            })
            .set_logged_in()
            .trips(3)
            .ais_vms_positions(9)
            .hauls(6)
            .landings(6)
            .build()
            .await;

        helper.app.login_user();
        let error = helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: state.trips.iter().map(|t| t.period.start()).min(),
                    end: state.trips.iter().map(|t| t.period.end()).max(),
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_org_benchmarks_works() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(3)
            .modify(|v| {
                v.fiskeridir.owners = vec![RegisterVesselOwner {
                    city: None,
                    entity_type: RegisterVesselEntityType::Company,
                    id: Some(org_id),
                    name: NonEmptyString::from_str("test").unwrap(),
                    postal_code: 9000,
                }];
            })
            .set_logged_in()
            .trips(3)
            .ais_vms_positions(9)
            .hauls(6)
            .landings(6)
            .build()
            .await;

        helper.app.login_user();
        let benchmarks = helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: state.trips.iter().map(|t| t.period.start()).min(),
                    end: state.trips.iter().map(|t| t.period.end()).max(),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_org_benchmarks(&benchmarks, &state.trips, &state.hauls, 3);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_org_benchmarks_filters_by_org() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(3)
            .modify_idx(|i, v| {
                if i < 2 {
                    v.fiskeridir.owners = vec![RegisterVesselOwner {
                        city: None,
                        entity_type: RegisterVesselEntityType::Company,
                        id: Some(org_id),
                        name: NonEmptyString::from_str("test").unwrap(),
                        postal_code: 9000,
                    }];
                }
            })
            .set_logged_in()
            .trips(3)
            .ais_vms_positions(9)
            .hauls(6)
            .landings(6)
            .build()
            .await;

        helper.app.login_user();
        let benchmarks = helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: state.trips.iter().map(|t| t.period.start()).min(),
                    end: state.trips.iter().map(|t| t.period.end()).max(),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_org_benchmarks(&benchmarks, &state.trips[1..], &state.hauls[2..], 2);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_org_benchmarks_filters_by_dates() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(3)
            .modify(|v| {
                v.fiskeridir.owners = vec![RegisterVesselOwner {
                    city: None,
                    entity_type: RegisterVesselEntityType::Company,
                    id: Some(org_id),
                    name: NonEmptyString::from_str("test").unwrap(),
                    postal_code: 9000,
                }];
            })
            .set_logged_in()
            .trips(3)
            .ais_vms_positions(9)
            .hauls(6)
            .landings(6)
            .build()
            .await;

        helper.app.login_user();
        let benchmarks = helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: Some(state.trips[1].period.start()),
                    end: state.trips.iter().map(|t| t.period.end()).max(),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_org_benchmarks(&benchmarks, &state.trips[1..], &state.hauls[2..], 2);
    })
    .await;
}

// Assumes all hauls and trips have not been modified beyond their test defaults
fn assert_org_benchmarks(
    benchmarks: &OrgBenchmarks,
    trips: &[TripDetailed],
    hauls: &[Haul],
    num_vessels: u64,
) {
    let trip_distance = trips.iter().map(|t| t.distance.unwrap()).sum::<f64>() as u64;
    let fishing_time = hauls
        .iter()
        .map(|t| t.duration().num_seconds())
        .sum::<i64>() as u64;
    let trip_time = trips
        .iter()
        .map(|t| t.period.duration().num_seconds())
        .sum::<i64>() as u64;
    let landing_weight = trips
        .iter()
        .map(|t| t.delivery.total_living_weight)
        .sum::<f64>() as u64;

    assert_eq!(trip_distance, benchmarks.trip_distance as u64);
    assert_eq!(fishing_time, benchmarks.fishing_time);
    assert_eq!(trip_time, benchmarks.trip_time);
    assert_eq!(
        landing_weight,
        benchmarks.landing_total_living_weight as u64
    );

    for b in benchmarks.vessels.iter().filter(|b| !b.is_empty()) {
        assert_eq!(b.trip_distance as u64, trip_distance / num_vessels);
        assert_eq!(b.fishing_time, fishing_time / num_vessels);
        assert_eq!(b.trip_time, trip_time / num_vessels);
        assert_eq!(
            b.landing_total_living_weight as u64,
            landing_weight / num_vessels
        );
    }
}
