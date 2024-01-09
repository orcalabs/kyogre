use crate::v1::helper::test;
use chrono::{Duration, Utc};
use reqwest::StatusCode;
use web_api::routes::v1::fuel::{
    DeleteFuelMeasurement, FuelMeasurement, FuelMeasurementBody, FuelMeasurementsParams,
};

#[tokio::test]
async fn test_cant_use_fuel_measurement_endpoints_without_bw_token() {
    test(|helper, _builder| async move {
        let response = helper
            .app
            .get_fuel_measurements(Default::default(), "".into())
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = vec![FuelMeasurementBody {
            timestamp: Utc::now(),
            fuel: 10.,
        }];

        let response = helper
            .app
            .create_fuel_measurements(body.clone(), "".into())
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = helper.app.update_fuel_measurements(body, "".into()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = helper
            .app
            .delete_fuel_measurements(
                vec![DeleteFuelMeasurement {
                    timestamp: Utc::now(),
                }],
                "".into(),
            )
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_create_and_get_fuel_measurement() {
    test(|helper, _builder| async move {
        let token = helper.bw_helper.get_bw_token();

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

        let response = helper
            .app
            .create_fuel_measurements(body, token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = helper
            .app
            .get_fuel_measurements(Default::default(), token)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let measurements: Vec<FuelMeasurement> = response.json().await.unwrap();
        assert_eq!(measurements.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_get_fuel_measurement_filters_by_dates() {
    test(|helper, _builder| async move {
        let token = helper.bw_helper.get_bw_token();

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

        let response = helper
            .app
            .create_fuel_measurements(body, token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let params = FuelMeasurementsParams {
            start_date: Some(now - Duration::days(2)),
            end_date: Some(now - Duration::days(1)),
        };

        let response = helper.app.get_fuel_measurements(params, token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let measurements: Vec<FuelMeasurement> = response.json().await.unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].fuel, 2000.);
    })
    .await;
}

#[tokio::test]
async fn test_update_fuel_measurement() {
    test(|helper, _builder| async move {
        let token = helper.bw_helper.get_bw_token();

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

        let response = helper
            .app
            .create_fuel_measurements(body.clone(), token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        for m in body.iter_mut() {
            m.fuel *= 10.;
        }

        let response = helper
            .app
            .update_fuel_measurements(body, token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = helper
            .app
            .get_fuel_measurements(Default::default(), token)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let measurements: Vec<FuelMeasurement> = response.json().await.unwrap();
        assert_eq!(measurements.len(), 3);
        assert_eq!(measurements[0].fuel, 10_000.);
        assert_eq!(measurements[1].fuel, 20_000.);
        assert_eq!(measurements[2].fuel, 30_000.);
    })
    .await;
}

#[tokio::test]
async fn test_delete_fuel_measurement() {
    test(|helper, _builder| async move {
        let token = helper.bw_helper.get_bw_token();

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

        let response = helper
            .app
            .create_fuel_measurements(body.clone(), token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = helper
            .app
            .get_fuel_measurements(Default::default(), token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let measurements: Vec<FuelMeasurement> = response.json().await.unwrap();
        assert_eq!(measurements.len(), 3);

        let delete = vec![
            DeleteFuelMeasurement {
                timestamp: body[0].timestamp,
            },
            DeleteFuelMeasurement {
                timestamp: body[2].timestamp,
            },
        ];

        let response = helper
            .app
            .delete_fuel_measurements(delete, token.clone())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = helper
            .app
            .get_fuel_measurements(Default::default(), token)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let measurements: Vec<FuelMeasurement> = response.json().await.unwrap();
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].fuel, 2000.);
    })
    .await;
}
