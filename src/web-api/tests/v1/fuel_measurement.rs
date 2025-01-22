use crate::v1::helper::test;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::GlobalLevel;
use engine::*;
use http_client::StatusCode;
use kyogre_core::{
    CreateFuelMeasurement, DeleteFuelMeasurement, FuelMeasurementId, ProcessingStatus,
};
use kyogre_core::{FuelMeasurement, FuelMeasurementRange, TestHelperOutbound};
use web_api::routes::v1::fuel_measurement::FuelMeasurementsParams;

#[tokio::test]
async fn test_cant_use_fuel_measurement_endpoints_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let body = &[CreateFuelMeasurement {
            timestamp: Utc::now(),
            fuel: 10.,
        }];

        let error = helper.app.create_fuel_measurements(body).await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let body = &[FuelMeasurement {
            id: FuelMeasurementId::test_new(1),
            timestamp: Utc::now(),
            fuel: 10.,
        }];

        let error = helper.app.update_fuel_measurements(body).await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let error = helper
            .app
            .delete_fuel_measurements(&[DeleteFuelMeasurement {
                id: FuelMeasurementId::test_new(765432),
            }])
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_create_returns_created_objects() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;

        helper.app.login_user();

        let now = Utc::now();

        let body = &[
            CreateFuelMeasurement {
                timestamp: now,
                fuel: 1000.,
            },
            CreateFuelMeasurement {
                timestamp: now - Duration::days(1),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: now - Duration::days(2),
                fuel: 3000.,
            },
        ];

        let measurements = helper.app.create_fuel_measurements(body).await.unwrap();

        assert_eq!(measurements.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_create_and_get_fuel_measurement() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;

        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 3);
        assert_eq!(ranges.len(), 2);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_get_fuel_measurement_filters_by_dates() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let params = FuelMeasurementsParams {
            start_date: Some(start + Duration::days(1)),
            end_date: Some(start + Duration::days(3)),
        };

        let measurements = helper.app.get_fuel_measurements(params).await.unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].fuel, 2000.);
    })
    .await;
}

#[tokio::test]
async fn test_update_fuel_measurement() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements.iter_mut().for_each(|m| m.fuel *= 10.);
        helper
            .app
            .update_fuel_measurements(&measurements)
            .await
            .unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        dbg!(&measurements);
        dbg!(&ranges);
        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel, 30_000.);
        assert_eq!(measurements[1].fuel, 20_000.);
        assert_eq!(measurements[2].fuel, 10_000.);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_delete_fuel_measurement() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel: 2500.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel: 1500.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        assert_eq!(measurements.len(), 5);
        measurements.sort_by_key(|m| m.timestamp);

        let delete = vec![
            DeleteFuelMeasurement {
                id: measurements[1].id,
            },
            DeleteFuelMeasurement {
                id: measurements[3].id,
            },
        ];

        helper.app.delete_fuel_measurements(&delete).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel, 3000.);
        assert_eq!(measurements[1].fuel, 2000.);
        assert_eq!(measurements[2].fuel, 1000.);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_create_splits_upper_and_lower_correctly() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(6),
                fuel: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel: 2500.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel: 1500.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 5);
        assert_eq!(ranges.len(), 4);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_create_handles_single_insert() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let body = vec![CreateFuelMeasurement {
            timestamp: Utc::now(),
            fuel: 1000.,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        assert_eq!(measurements.len(), 1);
        assert!(ranges.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_create_handles_later_insert() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![CreateFuelMeasurement {
            timestamp: start,
            fuel: 1000.,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let body = vec![CreateFuelMeasurement {
            timestamp: start + Duration::days(2),
            fuel: 500.,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 2);
        assert_eq!(ranges.len(), 1);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_create_handles_earlier_insert() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![CreateFuelMeasurement {
            timestamp: start,
            fuel: 1000.,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let body = vec![CreateFuelMeasurement {
            timestamp: start - Duration::days(2),
            fuel: 1500.,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 2);
        assert_eq!(ranges.len(), 1);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_delete_back_to_zero_entries() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel: 1000.,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        let delete = &[
            DeleteFuelMeasurement {
                id: measurements[0].id,
            },
            DeleteFuelMeasurement {
                id: measurements[1].id,
            },
            DeleteFuelMeasurement {
                id: measurements[2].id,
            },
        ];

        helper.app.delete_fuel_measurements(delete).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        assert!(measurements.is_empty());
        assert!(ranges.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_delete_back_to_one_entry() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel: 1000.,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        let delete = vec![
            DeleteFuelMeasurement {
                id: measurements[0].id,
            },
            DeleteFuelMeasurement {
                id: measurements[1].id,
            },
        ];

        helper.app.delete_fuel_measurements(&delete).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        assert_eq!(measurements.len(), 1);
        assert!(ranges.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_delete_back_to_two_entries() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;
        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel: 1000.,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        let delete = vec![DeleteFuelMeasurement {
            id: measurements[0].id,
        }];

        helper.app.delete_fuel_measurements(&delete).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 2);
        assert_eq!(ranges.len(), 1);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_creating_fuel_measurements_invalidates_trip_benchmark_status_for_overlapping_trips() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(3);

        let start2 = start + Duration::days(6);
        let end2 = start + Duration::days(9);

        let start3 = start + Duration::days(12);
        let end3 = start + Duration::days(15);

        builder
            .vessels(1)
            .set_engine_building_year()
            .set_logged_in()
            .trips(3)
            .modify_idx(|i, t| {
                let (start, end) = if i == 0 {
                    (start, end)
                } else if i == 1 {
                    (start2, end2)
                } else {
                    (start3, end3)
                };
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(30)
            .build()
            .await;

        helper.app.login_user();

        helper
            .app
            .create_fuel_measurements(&[
                CreateFuelMeasurement {
                    timestamp: start2 + Duration::days(2),
                    fuel: 3000.,
                },
                CreateFuelMeasurement {
                    timestamp: end3,
                    fuel: 2000.,
                },
            ])
            .await
            .unwrap();

        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Unprocessed)
                .await,
            2
        );
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Successful)
                .await,
            1
        );
    })
    .await;
}
#[tokio::test]
async fn test_deleting_fuel_measurements_invalidates_trip_benchmark_status_for_overlapping_trips() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(3);

        let start2 = start + Duration::days(6);
        let end2 = start + Duration::days(9);

        let start3 = start + Duration::days(12);
        let end3 = start + Duration::days(15);

        builder
            .vessels(1)
            .set_engine_building_year()
            .set_logged_in()
            .trips(3)
            .modify_idx(|i, t| {
                let (start, end) = if i == 0 {
                    (start, end)
                } else if i == 1 {
                    (start2, end2)
                } else {
                    (start3, end3)
                };
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(30)
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start2 + Duration::days(2),
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: end3,
                fuel: 2000.,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        helper.builder().await.build().await;
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Unprocessed)
                .await,
            0
        );
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Successful)
                .await,
            3
        );

        helper
            .app
            .delete_fuel_measurements(&[
                DeleteFuelMeasurement {
                    id: measurements[0].id,
                },
                DeleteFuelMeasurement {
                    id: measurements[1].id,
                },
            ])
            .await
            .unwrap();

        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Unprocessed)
                .await,
            2
        );
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Successful)
                .await,
            1
        );
    })
    .await;
}

#[tokio::test]
async fn test_updating_fuel_measurements_invalidates_trip_benchmark_status_for_overlapping_trips() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(3);

        let start2 = start + Duration::days(6);
        let end2 = start + Duration::days(9);

        let start3 = start + Duration::days(12);
        let end3 = start + Duration::days(15);

        builder
            .vessels(1)
            .set_engine_building_year()
            .set_logged_in()
            .trips(3)
            .modify_idx(|i, t| {
                let (start, end) = if i == 0 {
                    (start, end)
                } else if i == 1 {
                    (start2, end2)
                } else {
                    (start3, end3)
                };
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(30)
            .build()
            .await;

        helper.app.login_user();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start2 + Duration::days(2),
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: end3,
                fuel: 2000.,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        helper.builder().await.build().await;
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Unprocessed)
                .await,
            0
        );
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Successful)
                .await,
            3
        );

        measurements.iter_mut().for_each(|v| v.fuel *= 10.0);

        helper
            .app
            .update_fuel_measurements(&measurements)
            .await
            .unwrap();

        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Unprocessed)
                .await,
            2
        );
        assert_eq!(
            helper
                .adapter()
                .trips_with_benchmark_status(ProcessingStatus::Successful)
                .await,
            1
        );
    })
    .await;
}
fn assert_ranges_are_correct(measurements: &[FuelMeasurement], ranges: &[FuelMeasurementRange]) {
    for (i, r) in ranges.iter().enumerate() {
        let start = &measurements[i];
        let end = &measurements[i + 1];
        assert_eq!(r.fuel_range.start(), start.timestamp);
        assert_eq!(r.fuel_range.end(), end.timestamp);
        assert_eq!(r.fuel_used, start.fuel - end.fuel);
    }
}
