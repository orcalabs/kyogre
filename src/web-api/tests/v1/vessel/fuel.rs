use super::super::helper::test;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::*;
use float_cmp::approx_eq;
use kyogre_core::CreateFuelMeasurement;
use kyogre_core::TestHelperOutbound;
use web_api::routes::v1::vessel::FuelParams;

#[tokio::test]
async fn test_fuel_only_includes_measurments_within_given_range() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);
        let fuel_processor = builder.processors.estimator.clone();

        let _state = builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
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
                fuel_after_liter: None,
                fuel_liter: 4000.,
            },
            CreateFuelMeasurement {
                timestamp: start - Duration::days(8),
                fuel_after_liter: None,
                fuel_liter: 3000.,
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
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let estimate = helper
            .adapter()
            .sum_fuel_estimates(
                start.date_naive(),
                end.date_naive(),
                &[(start + Duration::days(3)).date_naive()],
            )
            .await;

        let expected = 1000.0 + estimate;

        assert!(estimate > 0.0);
        assert!(approx_eq!(f64, expected, fuel));
    })
    .await;
}

#[tokio::test]
async fn test_fuel_excludes_fuel_measurement_when_more_than_half_of_period_is_outside_range() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(10);

        builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
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
                fuel_after_liter: None,
                fuel_liter: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: end + Duration::days(5),
                fuel_after_liter: None,
                fuel_liter: 2000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        helper.builder().await.build().await;

        let vessel_fuel = helper
            .app
            .get_vessel_fuel(FuelParams {
                start_date: Some(start.date_naive()),
                end_date: Some(end.date_naive()),
            })
            .await
            .unwrap();

        let estimated_fuel = helper
            .adapter()
            .all_fuel_estimates()
            .await
            .into_iter()
            .sum();

        assert!(approx_eq!(f64, estimated_fuel, vessel_fuel));
    })
    .await;
}
