use super::helper::test;
use crate::v1::helper::overlap_factor;
use crate::v1::helper::overlap_factor_date;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::OrgId;
use float_cmp::approx_eq;
use kyogre_core::CreateFuelMeasurement;
use kyogre_core::DateRange;
use kyogre_core::TestHelperOutbound;
use kyogre_core::TEST_SIGNED_IN_VESSEL_CALLSIGN;
use web_api::routes::v1::vessel::FuelParams;

#[tokio::test]
async fn test_setting_fuel_after_computes_correct_fuel_used() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));
        let end = start + Duration::days(10);
        let org_id = OrgId::test_new(1);

        builder
            .vessels(1)
            .set_org_id_of_owner(org_id)
            .set_logged_in()
            .build()
            .await;
        helper.app.login_user();

        let body = &[
            CreateFuelMeasurement {
                timestamp: start,
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 2000.,
                fuel_after_liter: Some(2500.0),
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_liter: 1000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(body).await.unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let org_fuel = helper
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

        assert!(approx_eq!(f64, 2500.0, vessel_fuel));
        assert!(approx_eq!(f64, 2500.0, org_fuel));
    })
    .await;
}

// M---Q---Q---M
#[tokio::test]
async fn test_fuel_measurement_overlap_1() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let org_id = OrgId::test_new(1);
        let end = start + Duration::days(10);
        let start_date = start.date_naive();
        let end_date = end.date_naive();

        let first_measurement = start - Duration::hours(30);
        let last_measurement = end + Duration::hours(30);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .set_org_id_of_owner(org_id)
            .set_engine_building_year()
            .set_logged_in()
            .ais_vms_positions(10)
            .modify_idx(|i, p| {
                p.position
                    .set_timestamp(first_measurement + Duration::hours(i as i64));
            })
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .up()
            .ais_vms_positions(10)
            .modify_idx(|i, p| {
                p.position
                    .set_timestamp(last_measurement - Duration::hours(i as i64));
            })
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: first_measurement,
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: last_measurement,
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start_date),
                end_date: Some(end_date),
            })
            .await
            .unwrap();

        let org_fuel = helper
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

        assert!(approx_eq!(f64, 2000.0, vessel_fuel));
        assert!(approx_eq!(f64, 2000.0, org_fuel));
    })
    .await;
}

// M---Q---M---Q
#[tokio::test]
async fn test_fuel_measurement_overlap_2() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);
        let first_measurement = start - Duration::hours(4);
        let last_measurement = start + Duration::hours(40);
        let org_id = OrgId::test_new(1);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .set_org_id_of_owner(org_id)
            .set_engine_building_year()
            .set_logged_in()
            .ais_vms_positions(10)
            .modify_idx(|i, p| {
                p.position
                    .set_timestamp(first_measurement + Duration::minutes(i as i64));
            })
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .up()
            .ais_vms_positions(10)
            .modify_idx(|i, p| {
                p.position.set_timestamp(end + Duration::minutes(i as i64));
            })
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: first_measurement,
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: last_measurement,
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let org_fuel = helper
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

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert_eq!(ranges.len(), 1);

        let partially_covered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                last_measurement.date_naive(),
                last_measurement.date_naive(),
                &[],
                None,
            )
            .await;

        let uncovered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                last_measurement.date_naive().succ_opt().unwrap(),
                end.date_naive(),
                &[],
                None,
            )
            .await;

        let query_range = DateRange::from_dates(start.date_naive(), end.date_naive()).unwrap();

        let measurement_fuel = overlap_factor(
            first_measurement..=last_measurement,
            query_range.start()..query_range.end(),
        ) * 2000.0;

        let partially_covered_fuel = overlap_factor_date(
            last_measurement.date_naive()..=last_measurement.date_naive(),
            first_measurement..last_measurement,
        ) * partially_covered_estimate;

        let expected = measurement_fuel + uncovered_estimate + partially_covered_fuel;

        assert!(measurement_fuel > 0.0);
        assert!(uncovered_estimate > 0.0);
        assert!(partially_covered_fuel > 0.0);
        assert!(approx_eq!(f64, expected, vessel_fuel));
        assert!(approx_eq!(f64, expected, org_fuel));
    })
    .await;
}

// Q---M---Q---M
#[tokio::test]
async fn test_fuel_measurement_overlap_3() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);
        let first_measurement = end - Duration::hours(40);
        let last_measurement = end + Duration::hours(30);
        let org_id = OrgId::test_new(1);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .set_org_id_of_owner(org_id)
            .set_engine_building_year()
            .set_logged_in()
            .ais_vms_positions(10)
            .modify_idx(|i, p| {
                p.position
                    .set_timestamp(first_measurement + Duration::minutes(i as i64));
            })
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .up()
            .ais_vms_positions(10)
            .modify_idx(|i, p| {
                p.position.set_timestamp(end + Duration::minutes(i as i64));
            })
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: first_measurement,
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: last_measurement,
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let org_fuel = helper
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

        let partially_covered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                first_measurement.date_naive(),
                first_measurement.date_naive(),
                &[],
                None,
            )
            .await;

        let uncovered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                start.date_naive(),
                first_measurement.date_naive().pred_opt().unwrap(),
                &[],
                None,
            )
            .await;

        let query_range = DateRange::from_dates(start.date_naive(), end.date_naive()).unwrap();

        let measurement_fuel = overlap_factor(
            first_measurement..=last_measurement,
            query_range.start()..query_range.end(),
        ) * 2000.0;

        let partially_covered_fuel = overlap_factor_date(
            first_measurement.date_naive()..=first_measurement.date_naive(),
            first_measurement..last_measurement,
        ) * partially_covered_estimate;

        let expected = measurement_fuel + uncovered_estimate + partially_covered_fuel;

        assert!(measurement_fuel > 0.0);
        assert!(uncovered_estimate > 0.0);
        assert!(partially_covered_fuel > 0.0);
        assert!(approx_eq!(f64, expected, vessel_fuel));
        assert!(approx_eq!(f64, expected, org_fuel));
    })
    .await;
}

// Q---M---M---Q
#[tokio::test]
async fn test_fuel_measurement_overlap_4() {
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
            .set_org_id_of_owner(org_id)
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

        let first_measurement = start + Duration::hours(4);
        let last_measurement = start + Duration::hours(6);

        let body = vec![
            CreateFuelMeasurement {
                timestamp: first_measurement,
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: last_measurement,
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let org_fuel = helper
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

        let partially_covered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                first_measurement.date_naive(),
                last_measurement.date_naive(),
                &[],
                None,
            )
            .await;

        let uncovered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                start.date_naive(),
                end.date_naive(),
                &[first_measurement.date_naive()],
                None,
            )
            .await;

        let partially_covered_fuel = overlap_factor_date(
            first_measurement.date_naive()..=last_measurement.date_naive(),
            first_measurement..last_measurement,
        ) * partially_covered_estimate;

        let expected = 2000.0 + uncovered_estimate + partially_covered_fuel;

        assert!(uncovered_estimate > 0.0);
        assert!(partially_covered_fuel > 0.0);
        assert!(approx_eq!(f64, expected, vessel_fuel));
        assert!(approx_eq!(f64, expected, org_fuel));
    })
    .await;
}

// Q--D(s)--M--M--M--M--D(e)--Q
#[tokio::test]
async fn test_fuel_measurement_overlap_5() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let org_id = OrgId::test_new(1);
        let end = start + Duration::days(10);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .set_org_id_of_owner(org_id)
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

        let first_measurement = start + Duration::hours(4);
        let second_measurement = start + Duration::hours(10);
        let third_measurement = start + Duration::hours(15);
        let last_measurement = start + Duration::hours(20);

        let body = vec![
            CreateFuelMeasurement {
                timestamp: first_measurement,
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: second_measurement,
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: third_measurement,
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: last_measurement,
                fuel_liter: 1000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let org_fuel = helper
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

        let partially_covered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                first_measurement.date_naive(),
                last_measurement.date_naive(),
                &[],
                None,
            )
            .await;

        let uncovered_estimate = helper
            .adapter()
            .sum_fuel_estimates(
                start.date_naive(),
                end.date_naive(),
                &[first_measurement.date_naive()],
                None,
            )
            .await;

        let partially_covered_fuel = overlap_factor_date(
            first_measurement.date_naive()..=last_measurement.date_naive(),
            first_measurement..last_measurement,
        ) * partially_covered_estimate;

        let expected = 3000.0 + uncovered_estimate + partially_covered_fuel;

        assert!(uncovered_estimate > 0.0);
        assert!(partially_covered_fuel > 0.0);
        assert!(approx_eq!(f64, expected, vessel_fuel));
        assert!(approx_eq!(f64, expected, org_fuel));
    })
    .await;
}

#[tokio::test]
async fn test_fuel_excludes_non_active_vessels() {
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

        let estimate = helper
            .adapter()
            .sum_fuel_estimates(
                start.date_naive(),
                end.date_naive(),
                &[],
                Some(&[state.vessels[0].fiskeridir.id]),
            )
            .await;

        let org_fuel = helper
            .app
            .get_org_fuel(
                org_id,
                FuelParams {
                    start_date: Some(start.date_naive()),
                    end_date: Some(end.date_naive()),
                },
            )
            .await
            .unwrap();

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        assert_eq!(org_fuel.len(), 1);
        assert_eq!(
            org_fuel[0].fiskeridir_vessel_id,
            state.vessels[0].fiskeridir.id
        );
        assert!(approx_eq!(f64, estimate, org_fuel[0].estimated_fuel_liter));
        assert!(approx_eq!(f64, estimate, vessel_fuel));
    })
    .await;
}
