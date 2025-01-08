use crate::v1::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::{CallSign, GearGroup};
use float_cmp::approx_eq;
use http_client::StatusCode;
use kyogre_core::{FiskeridirVesselId, TEST_SIGNED_IN_VESSEL_CALLSIGN};
use web_api::routes::v1::fuel::{
    DeleteFuelMeasurement, FuelMeasurementBody, FuelMeasurementsParams, FuelParams,
};

#[tokio::test]
async fn test_cant_use_fuel_endpoint_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper.app.get_fuel(Default::default()).await.unwrap_err();
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
            .get_fuel(FuelParams {
                start_date: Some(state.ais_vms_positions[0].timestamp.naive_utc().date()),
                end_date: Some(state.ais_vms_positions[9].timestamp.naive_utc().date()),
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
            .get_fuel(FuelParams {
                start_date: Some(state.ais_vms_positions[0].timestamp.naive_utc().date()),
                end_date: Some(state.ais_vms_positions[9].timestamp.naive_utc().date()),
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

        let fuel = helper.app.get_fuel(Default::default()).await.unwrap();
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
            .get_fuel(FuelParams {
                start_date: Some(state.vms_positions[0].timestamp.naive_utc().date()),
                end_date: Some(state.vms_positions[9].timestamp.naive_utc().date()),
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
            .get_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
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
            .get_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
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
            .get_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        assert!(!approx_eq!(f64, fuel, fuel2))
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
            .get_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
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
            .get_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        assert!(!approx_eq!(f64, fuel, fuel2))
    })
    .await;
}

#[tokio::test]
async fn test_cant_use_fuel_measurement_endpoints_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let body = vec![FuelMeasurementBody {
            timestamp: Utc::now(),
            fuel: 10.,
        }];

        let error = helper
            .app
            .create_fuel_measurements(body.clone())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let error = helper.app.update_fuel_measurements(body).await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);

        let error = helper
            .app
            .delete_fuel_measurements(vec![DeleteFuelMeasurement {
                timestamp: Utc::now(),
            }])
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_create_and_get_fuel_measurement() {
    test(|mut helper, _builder| async move {
        helper.app.login_user();

        let now = Utc::now();

        let body = vec![
            FuelMeasurementBody {
                timestamp: now,
                fuel: 1000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(1),
                fuel: 2000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(2),
                fuel: 3000.,
            },
        ];

        helper.app.create_fuel_measurements(body).await.unwrap();

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        assert_eq!(measurements.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_get_fuel_measurement_filters_by_dates() {
    test(|mut helper, _builder| async move {
        helper.app.login_user();

        let now = Utc::now();

        let body = vec![
            FuelMeasurementBody {
                timestamp: now,
                fuel: 1000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(2),
                fuel: 2000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(4),
                fuel: 3000.,
            },
        ];

        helper.app.create_fuel_measurements(body).await.unwrap();

        let params = FuelMeasurementsParams {
            start_date: Some(now - Duration::days(2)),
            end_date: Some(now - Duration::days(1)),
        };

        let measurements = helper.app.get_fuel_measurements(params).await.unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].fuel, 2000.);
    })
    .await;
}

#[tokio::test]
async fn test_update_fuel_measurement() {
    test(|mut helper, _builder| async move {
        helper.app.login_user();

        let now = Utc::now();

        let mut body = vec![
            FuelMeasurementBody {
                timestamp: now,
                fuel: 1000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(2),
                fuel: 2000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(4),
                fuel: 3000.,
            },
        ];

        helper
            .app
            .create_fuel_measurements(body.clone())
            .await
            .unwrap();

        for m in body.iter_mut() {
            m.fuel *= 10.;
        }

        helper.app.update_fuel_measurements(body).await.unwrap();

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel, 10_000.);
        assert_eq!(measurements[1].fuel, 20_000.);
        assert_eq!(measurements[2].fuel, 30_000.);
    })
    .await;
}

#[tokio::test]
async fn test_delete_fuel_measurement() {
    test(|mut helper, _builder| async move {
        helper.app.login_user();

        let now = Utc::now();

        let body = vec![
            FuelMeasurementBody {
                timestamp: now,
                fuel: 1000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(2),
                fuel: 2000.,
            },
            FuelMeasurementBody {
                timestamp: now - Duration::days(4),
                fuel: 3000.,
            },
        ];

        helper
            .app
            .create_fuel_measurements(body.clone())
            .await
            .unwrap();

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        assert_eq!(measurements.len(), 3);

        let delete = vec![
            DeleteFuelMeasurement {
                timestamp: body[0].timestamp,
            },
            DeleteFuelMeasurement {
                timestamp: body[2].timestamp,
            },
        ];

        helper.app.delete_fuel_measurements(delete).await.unwrap();

        let measurements = helper
            .app
            .get_fuel_measurements(Default::default())
            .await
            .unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].fuel, 2000.);
    })
    .await;
}
