use super::helper::test;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::{
    NonEmptyString, OrgId, RegisterVesselEntityType, RegisterVesselOwner, SpeciesGroup,
};
use float_cmp::approx_eq;
use http_client::StatusCode;
use kyogre_core::{CreateFuelMeasurement, TestHelperOutbound, TEST_SIGNED_IN_VESSEL_CALLSIGN};
use kyogre_core::{Haul, OrgBenchmarks, TripDetailed};
use std::str::FromStr;
use web_api::routes::v1::{org::OrgBenchmarkParameters, user::User, vessel::FuelParams};

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
async fn test_vessel_org_benchmarks_works_with_trips_with_different_amount_of_landings() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(1)
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
            .trips(2)
            .landings(3)
            .build()
            .await;

        helper.app.login_user();
        helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: state.trips.iter().map(|t| t.period.start()).min(),
                    end: state.trips.iter().map(|t| t.period.end()).max(),
                },
            )
            .await
            .unwrap();
    })
    .await;
}
#[tokio::test]
async fn test_vessel_org_benchmarks_works_with_trips_without_landings() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(2)
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
            .trips(2)
            .build()
            .await;

        helper.app.login_user();
        helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: state.trips.iter().map(|t| t.period.start()).min(),
                    end: state.trips.iter().map(|t| t.period.end()).max(),
                },
            )
            .await
            .unwrap();
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
async fn test_vessel_org_benchmarks_sums_species_per_vessel() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(2)
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
            .trips(4)
            .landings(8)
            .modify_idx(|i, l| {
                let species = if i % 2 == 0 {
                    SpeciesGroup::Seabird
                } else {
                    SpeciesGroup::SharkFish
                };
                l.landing.product.species.group_code = species;
                l.landing.product.living_weight = Some(50.0);
            })
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

        assert_eq!(benchmarks.vessels.len(), 2);
        for b in benchmarks.vessels {
            assert_eq!(b.species.len(), 2);

            assert_eq!(b.species[0].species_group_id, SpeciesGroup::SharkFish);
            assert_eq!(b.species[0].landing_total_living_weight as u32, 100);

            assert_eq!(b.species[1].species_group_id, SpeciesGroup::Seabird);
            assert_eq!(b.species[1].landing_total_living_weight as u32, 100);
        }
    })
    .await;
}
#[tokio::test]
async fn test_vessel_org_benchmarks_works() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(3)
            .set_org_id_of_owner(org_id)
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
#[tokio::test]
async fn test_org_fuel_returns_not_found_for_non_logged_in_users() {
    test(|helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let state = builder
            .vessels(1)
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
            .trips(1)
            .ais_vms_positions(10)
            .build()
            .await;

        let error = helper
            .app
            .get_org_fuel(
                org_id,
                FuelParams {
                    start_date: state
                        .trips
                        .iter()
                        .map(|t| t.period.start().date_naive())
                        .min(),
                    end_date: state
                        .trips
                        .iter()
                        .map(|t| t.period.end().date_naive())
                        .max(),
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_get_org_fuel_for_org_user_is_not_part_of_returns_not_found() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let org_id2 = OrgId::test_new(2);
        let state = builder
            .vessels(3)
            .modify_idx(|i, v| {
                let org_id = if i > 1 { org_id2 } else { org_id };
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
            .ais_vms_positions(18)
            .build()
            .await;

        helper.app.login_user();

        let error = helper
            .app
            .get_org_fuel(
                org_id2,
                FuelParams {
                    start_date: state
                        .trips
                        .iter()
                        .map(|t| t.period.start().date_naive())
                        .min(),
                    end_date: state
                        .trips
                        .iter()
                        .map(|t| t.period.end().date_naive())
                        .max(),
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_org_fuel_filter_by_orgs() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        let org_id2 = OrgId::test_new(2);
        let state = builder
            .vessels(3)
            .set_engine_building_year()
            .modify_idx(|i, v| {
                let org_id = if i > 1 { org_id2 } else { org_id };
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
            .ais_vms_positions(18)
            .build()
            .await;

        helper.app.login_user();

        let fuel = helper
            .app
            .get_org_fuel(
                org_id,
                FuelParams {
                    start_date: state
                        .trips
                        .iter()
                        .map(|t| t.period.start().date_naive())
                        .min(),
                    end_date: state
                        .trips
                        .iter()
                        .map(|t| t.period.end().date_naive())
                        .max(),
                },
            )
            .await
            .unwrap();

        let expected_fuel = helper
            .adapter()
            .all_fuel_estimates()
            .await
            .into_iter()
            .sum::<f64>()
            / 3.0;

        assert_eq!(fuel.len(), 2);
        assert!(approx_eq!(f64, expected_fuel, fuel[0].estimated_fuel_liter));
        assert!(approx_eq!(f64, expected_fuel, fuel[1].estimated_fuel_liter));
        assert!(!fuel
            .iter()
            .any(|f| f.fiskeridir_vessel_id == state.vessels[2].fiskeridir.id));
    })
    .await;
}
#[tokio::test]
async fn test_org_fuel_returns_empty_response_with_no_data() {
    test(|mut helper, builder| async move {
        let org_id = OrgId::test_new(1);
        builder
            .vessels(1)
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
            .build()
            .await;

        helper.app.login_user();

        let now = Utc::now();

        let fuel = helper
            .app
            .get_org_fuel(
                org_id,
                FuelParams {
                    start_date: Some((now - Duration::days(10)).date_naive()),
                    end_date: Some(now.date_naive()),
                },
            )
            .await
            .unwrap();

        assert!(fuel.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_org_fuel_only_includes_measurments_within_given_range() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);
        let fuel_processor = builder.processors.estimator.clone();

        let org_id = OrgId::test_new(1);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.owners = vec![RegisterVesselOwner {
                    city: None,
                    entity_type: RegisterVesselEntityType::Company,
                    id: Some(org_id),
                    name: NonEmptyString::from_str("test").unwrap(),
                    postal_code: 9000,
                }];
            })
            .set_engine_building_year()
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start - Duration::days(10),
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start - Duration::days(8),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel_liter: 1000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        fuel_processor.run_single(None).await.unwrap();

        let fuel = helper
            .app
            .get_org_fuel(
                org_id,
                FuelParams {
                    start_date: Some(start.date_naive()),
                    end_date: Some(end.date_naive()),
                },
            )
            .await
            .unwrap()
            .iter()
            .map(|v| v.estimated_fuel_liter)
            .sum();

        let estimate = helper
            .adapter()
            .sum_fuel_estimates(
                start.date_naive(),
                end.date_naive(),
                &[(start + Duration::days(3)).date_naive()],
                None,
            )
            .await;

        let expected = 1000.0 + estimate;

        assert!(estimate > 0.0);
        assert!(approx_eq!(f64, expected, fuel));
    })
    .await;
}

#[tokio::test]
async fn test_org_fuel_excludes_fuel_measurement_when_more_than_half_of_period_is_outside_range() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);
        let org_id = OrgId::test_new(1);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.owners = vec![RegisterVesselOwner {
                    city: None,
                    entity_type: RegisterVesselEntityType::Company,
                    id: Some(org_id),
                    name: NonEmptyString::from_str("test").unwrap(),
                    postal_code: 9000,
                }];
            })
            .set_engine_building_year()
            .set_logged_in()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start + Duration::days(7),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: end + Duration::days(5),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        helper.builder().await.build().await;

        let fuel = helper
            .app
            .get_org_fuel(
                org_id,
                FuelParams {
                    start_date: Some(start.date_naive()),
                    end_date: Some(end.date_naive()),
                },
            )
            .await
            .unwrap()
            .iter()
            .map(|v| v.estimated_fuel_liter)
            .sum();

        let estimated_fuel = helper
            .adapter()
            .all_fuel_estimates()
            .await
            .into_iter()
            .sum();

        assert!(approx_eq!(f64, estimated_fuel, fuel));
    })
    .await;
}

#[tokio::test]
async fn test_org_benchmarks_excludes_non_active_vessels() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);
        let org_id = OrgId::test_new(1);

        let state = builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .set_org_id_of_owner(org_id)
            .set_engine_building_year()
            .set_logged_in()
            .active_vessel()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .up()
            .vessels(1)
            .set_org_id_of_owner(org_id)
            .set_engine_building_year()
            .set_call_sign(&(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap()))
            .historic_vessel()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .build()
            .await;

        helper.app.login_user();

        helper.builder().await.build().await;

        let benchmarks = helper
            .app
            .get_org_benchmarks(
                org_id,
                OrgBenchmarkParameters {
                    start: Some(start),
                    end: Some(end),
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(benchmarks.vessels.len(), 1);
        assert_eq!(
            benchmarks.vessels[0].fiskeridir_vessel_id,
            state.vessels[0].fiskeridir.id
        );
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
