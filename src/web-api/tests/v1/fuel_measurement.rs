use crate::v1::helper::test;
use chrono::{Duration, Utc};
use http_client::StatusCode;
use web_api::routes::v1::fuel_measurement::{
    DeleteFuelMeasurement, FuelMeasurementBody, FuelMeasurementsParams,
};

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
