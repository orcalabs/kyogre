use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::*;
use http_client::StatusCode;
use kyogre_core::{
    CreateFuelMeasurement, DeleteFuelMeasurement, FuelMeasurement, FuelMeasurementId,
    FuelMeasurementRange, OptionalDateTimeRange, ProcessingStatus, TestHelperOutbound,
};
use web_api::{
    error::ErrorDiscriminants,
    routes::v1::fuel_measurement::{FuelMeasurementsParams, UploadFuelMeasurement},
};

use crate::v1::helper::test;

#[tokio::test]
async fn test_cant_use_fuel_measurement_endpoints_without_being_associated_with_a_vessel() {
    test(|mut helper, _builder| async move {
        helper.app.login_user();
        let body = &[CreateFuelMeasurement {
            timestamp: Utc::now(),
            fuel_liter: 10.,
            fuel_after_liter: None,
        }];

        let error = helper.app.create_fuel_measurements(body).await.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::CallSignDoesNotExist);

        let body = &[FuelMeasurement {
            id: FuelMeasurementId::test_new(1),
            timestamp: Utc::now(),
            fuel_liter: 10.,
            fuel_after_liter: None,
        }];

        let error = helper.app.update_fuel_measurements(body).await.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::CallSignDoesNotExist);

        let error = helper
            .app
            .delete_fuel_measurements(&[DeleteFuelMeasurement {
                id: FuelMeasurementId::test_new(765432),
            }])
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::CallSignDoesNotExist);
    })
    .await;
}

// TODO: fix flickering test, error ouptput below
// thread 'v1::fuel_measurement::test_cant_use_fuel_measurement_endpoints_without_bw_token' (140890) panicked at web-api/tests/v1/test_client.rs:376:13:
// request failure: 0: HTTP reqwest error, at /home/jon/workspace/kyogre/src/http-client/src/request.rs:41:24
// 1: Middleware(error sending request for url (http://127.0.0.1:37559/v1.0/fuel_measurements)
//
// Caused by:
//     0: client error (SendRequest)
//     1: connection closed before message completed)
// note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
//
// #[tokio::test]
// async fn test_cant_use_fuel_measurement_endpoints_without_bw_token() {
//     test(|helper, _builder| async move {
//         let error = helper
//             .app
//             .get_fuel_measurements(Default::default())
//             .await
//             .unwrap_err();
//         assert_eq!(error.status, StatusCode::NOT_FOUND);
//
//         let body = &[CreateFuelMeasurement {
//             timestamp: Utc::now(),
//             fuel_liter: 10.,
//             fuel_after_liter: None,
//         }];
//
//         let error = helper.app.create_fuel_measurements(body).await.unwrap_err();
//         assert_eq!(error.status, StatusCode::NOT_FOUND);
//
//         let body = &[FuelMeasurement {
//             id: FuelMeasurementId::test_new(1),
//             timestamp: Utc::now(),
//             fuel_liter: 10.,
//             fuel_after_liter: None,
//         }];
//
//         let error = helper.app.update_fuel_measurements(body).await.unwrap_err();
//         assert_eq!(error.status, StatusCode::NOT_FOUND);
//
//         let error = helper
//             .app
//             .delete_fuel_measurements(&[DeleteFuelMeasurement {
//                 id: FuelMeasurementId::test_new(765432),
//             }])
//             .await
//             .unwrap_err();
//         assert_eq!(error.status, StatusCode::NOT_FOUND);
//     })
//     .await;
// }

#[tokio::test]
async fn test_create_returns_created_objects() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;

        helper.app.login_user();

        let now = Utc::now();

        let body = &[
            CreateFuelMeasurement {
                timestamp: now,
                fuel_liter: 1000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: now - Duration::days(1),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: now - Duration::days(2),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
        ];

        let measurements = helper.app.create_fuel_measurements(body).await.unwrap();
        assert_eq!(measurements.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_upload_returns_uploaded_objects() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;

        helper.app.login_user();

        let bytes = include_bytes!("../Fuel.xlsx");
        let file = BASE64_STANDARD.encode(bytes);

        let body = UploadFuelMeasurement { file };

        let measurements = helper.app.upload_fuel_measurements(body).await.unwrap();
        assert_eq!(measurements.len(), 10);
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                fuel_after_liter: None,
                timestamp: start + Duration::days(2),
                fuel_liter: 1000.,
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
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

        let params = FuelMeasurementsParams {
            range: OptionalDateTimeRange::test_new(
                Some(start + Duration::days(1)),
                Some(start + Duration::days(3)),
            ),
        };

        let measurements = helper.app.get_fuel_measurements(params).await.unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].fuel_liter, 2000.);
    })
    .await;
}

#[tokio::test]
async fn test_update_fuel_measurement_only_update_fuel() {
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
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

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements.iter_mut().for_each(|m| m.fuel_liter *= 10.);
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

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel_liter, 30_000.);
        assert_eq!(measurements[1].fuel_liter, 20_000.);
        assert_eq!(measurements[2].fuel_liter, 10_000.);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_update_mulitlpe_fuel_measurement_move_timestamp_within_existing_fuel_measurement_range()
 {
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
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

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements[1].timestamp = start + Duration::days(3);

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

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel_liter, 3000.);
        assert_eq!(measurements[1].fuel_liter, 2000.);
        assert_eq!(measurements[2].fuel_liter, 1000.);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_update_single_fuel_measurement_move_timestamp_within_existing_fuel_measurement_range()
{
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_after_liter: None,
                fuel_liter: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel_after_liter: None,
                fuel_liter: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements[1].timestamp = start + Duration::days(3);

        helper
            .app
            .update_fuel_measurements(&measurements[1..=1])
            .await
            .unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel_liter, 3000.);
        assert_eq!(measurements[1].fuel_liter, 2000.);
        assert_eq!(measurements[2].fuel_liter, 1000.);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}
#[tokio::test]
async fn test_update_mulitple_fuel_measurement_move_timestamp_outside_existing_fuel_measurement_range()
 {
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_after_liter: None,
                fuel_liter: 2000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel_after_liter: None,
                fuel_liter: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements[1].timestamp = start + Duration::days(6);
        measurements[1].fuel_liter = 500.;

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

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel_liter, 3000.);
        assert_eq!(measurements[1].fuel_liter, 1000.);
        assert_eq!(measurements[2].fuel_liter, 500.);
        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_update_single_fuel_measurement_move_timestamp_outside_existing_fuel_measurement_range()
 {
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel_after_liter: None,
                fuel_liter: 1000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();

        measurements[1].timestamp = start + Duration::days(6);
        measurements[1].fuel_liter = 500.;

        helper
            .app
            .update_fuel_measurements(&measurements[1..=1])
            .await
            .unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel_liter, 3000.);
        assert_eq!(measurements[1].fuel_liter, 1000.);
        assert_eq!(measurements[2].fuel_liter, 500.);
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_liter: 2500.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel_liter: 1500.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel_after_liter: None,
                fuel_liter: 1000.,
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
        assert_eq!(measurements[0].fuel_liter, 3000.);
        assert_eq!(measurements[1].fuel_liter, 2000.);
        assert_eq!(measurements[2].fuel_liter, 1000.);
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(6),
                fuel_liter: 1000.,
                fuel_after_liter: None,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let body = vec![
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_liter: 2500.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(4),
                fuel_liter: 1500.,
                fuel_after_liter: None,
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
            fuel_liter: 1000.,
            fuel_after_liter: None,
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
            fuel_after_liter: None,
            fuel_liter: 1000.,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let body = vec![CreateFuelMeasurement {
            timestamp: start + Duration::days(2),
            fuel_liter: 500.,
            fuel_after_liter: None,
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
            fuel_liter: 1000.,
            fuel_after_liter: None,
        }];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        let body = vec![CreateFuelMeasurement {
            timestamp: start - Duration::days(2),
            fuel_liter: 1500.,
            fuel_after_liter: None,
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel_liter: 1000.,
                fuel_after_liter: None,
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel_liter: 1000.,
                fuel_after_liter: None,
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(3),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(5),
                fuel_liter: 1000.,
                fuel_after_liter: None,
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
                    fuel_liter: 3000.,
                    fuel_after_liter: None,
                },
                CreateFuelMeasurement {
                    timestamp: end3,
                    fuel_liter: 2000.,
                    fuel_after_liter: None,
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: end3,
                fuel_liter: 2000.,
                fuel_after_liter: None,
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: end3,
                fuel_liter: 2000.,
                fuel_after_liter: None,
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

        measurements.iter_mut().for_each(|v| v.fuel_liter *= 10.0);

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

#[tokio::test]
async fn test_delete_fuel_measurement_sets_fuel_after_on_new_fuel_measurement_range() {
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
                fuel_liter: 3000.,
                fuel_after_liter: Some(5000.0),
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 2000.,
                fuel_after_liter: Some(4000.0),
            },
            CreateFuelMeasurement {
                fuel_after_liter: None,
                timestamp: start + Duration::days(2),
                fuel_liter: 1000.0,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        let delete = vec![DeleteFuelMeasurement {
            id: measurements[1].id,
        }];

        helper.app.delete_fuel_measurements(&delete).await.unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        assert_eq!(measurements.len(), 2);
        assert_eq!(measurements[0].fuel_after_liter, Some(5000.0));

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].fuel_used_liter, 4000.0);
    })
    .await;
}

#[tokio::test]
async fn test_update_fuel_measurement_with_timestamp_outside_exisiting_fuel_measurment_range_sets_fuel_after_on_new_fuel_measurement_range()
 {
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
                fuel_liter: 3000.,
                fuel_after_liter: Some(5000.0),
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 2000.,
                fuel_after_liter: Some(4000.0),
            },
            CreateFuelMeasurement {
                fuel_after_liter: None,
                timestamp: start + Duration::days(2),
                fuel_liter: 1000.0,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);

        measurements[1].timestamp = start + Duration::days(4);
        measurements[1].fuel_after_liter = Some(2000.0);
        measurements[1].fuel_liter = 500.0;

        helper
            .app
            .update_fuel_measurements(&measurements[1..=1])
            .await
            .unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        let ranges = helper.adapter().all_fuel_measurement_ranges().await;

        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel_after_liter, Some(5000.0));
        assert_eq!(measurements[1].fuel_after_liter, None);
        assert_eq!(measurements[2].fuel_after_liter, Some(2000.0));

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].fuel_used_liter, 4000.0);
        assert_eq!(ranges[1].fuel_used_liter, 500.0);
    })
    .await;
}
#[tokio::test]
async fn test_update_fuel_measurement_with_timestamp_outside_exisiting_fuel_measurment_range_but_inside_another_range()
 {
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
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                fuel_after_liter: None,
                timestamp: start + Duration::days(2),
                fuel_liter: 2000.0,
            },
            CreateFuelMeasurement {
                fuel_after_liter: None,
                timestamp: start + Duration::days(4),
                fuel_liter: 1000.0,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        assert_eq!(measurements.len(), 4);

        measurements[1].timestamp = start + Duration::days(3);
        measurements[1].fuel_liter = 1500.0;

        helper
            .app
            .update_fuel_measurements(&measurements[1..=1])
            .await
            .unwrap();

        let mut measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert_eq!(ranges.len(), 3);

        assert_ranges_are_correct(&measurements, &ranges);
    })
    .await;
}

#[tokio::test]
async fn test_create_greater_fuel_after_than_fuel_returns_bad_request() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;

        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![CreateFuelMeasurement {
            timestamp: start,
            fuel_liter: 3000.,
            fuel_after_liter: Some(1000.0),
        }];

        let err = helper
            .app
            .create_fuel_measurements(&body)
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.error, ErrorDiscriminants::FuelAfterLowerThanFuel);
    })
    .await;
}

#[tokio::test]
async fn test_update_greater_fuel_after_than_fuel_returns_bad_request() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().build().await;

        helper.app.login_user();

        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let body = vec![CreateFuelMeasurement {
            timestamp: start,
            fuel_liter: 3000.,
            fuel_after_liter: Some(4000.0),
        }];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements[0].fuel_after_liter = Some(2000.0);

        let err = helper
            .app
            .update_fuel_measurements(&measurements)
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.error, ErrorDiscriminants::FuelAfterLowerThanFuel);
    })
    .await;
}

#[tokio::test]
async fn test_create_fuel_measurements_with_fuel_used_equal_or_lower_to_zero_does_not_create_fuel_measurement_range()
 {
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
        ];

        let measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        assert_eq!(measurements.len(), 2);

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert!(ranges.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_update_fuel_measurements_with_fuel_used_equal_or_lower_than_zero_does_not_create_fuel_measurement_range()
 {
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        assert_eq!(measurements.len(), 2);

        measurements[1].fuel_liter = 3000.0;

        helper
            .app
            .update_fuel_measurements(&measurements)
            .await
            .unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert!(ranges.is_empty());
    })
    .await;
}
#[tokio::test]
async fn test_update_fuel_measurements_to_outside_existing_range_with_fuel_used_equal_or_lower_than_zero_does_not_create_fuel_measurement_range()
 {
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
                fuel_liter: 4000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_liter: 2000.,
                fuel_after_liter: None,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        assert_eq!(measurements.len(), 3);

        measurements[1].fuel_liter = 2000.0;
        measurements[1].timestamp = start + Duration::days(4);

        helper
            .app
            .update_fuel_measurements(&measurements)
            .await
            .unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].fuel_used_liter, 2000.0);
        assert_eq!(ranges[0].fuel_range.start(), start);
        assert_eq!(ranges[0].fuel_range.end(), start + Duration::days(2));
    })
    .await;
}

#[tokio::test]
async fn test_delete_fuel_measurements_with_fuel_used_equal_or_lower_than_zero_does_not_create_fuel_measurement_range()
 {
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
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(1),
                fuel_liter: 2500.,
                fuel_after_liter: None,
            },
            CreateFuelMeasurement {
                timestamp: start + Duration::days(2),
                fuel_liter: 3000.,
                fuel_after_liter: None,
            },
        ];

        let mut measurements = helper.app.create_fuel_measurements(&body).await.unwrap();
        measurements.sort_by_key(|m| m.timestamp);
        assert_eq!(measurements.len(), 3);

        helper
            .app
            .delete_fuel_measurements(&[DeleteFuelMeasurement {
                id: measurements[1].id,
            }])
            .await
            .unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert!(ranges.is_empty());
    })
    .await;
}

fn assert_ranges_are_correct(measurements: &[FuelMeasurement], ranges: &[FuelMeasurementRange]) {
    for (i, r) in ranges.iter().enumerate() {
        let start = &measurements[i];
        let end = &measurements[i + 1];
        assert_eq!(r.fuel_range.start(), start.timestamp);
        assert_eq!(r.fuel_range.end(), end.timestamp);
        assert_eq!(r.fuel_used_liter, start.fuel_liter - end.fuel_liter);
    }
}
