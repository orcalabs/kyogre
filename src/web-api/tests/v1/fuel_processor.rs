use super::helper::test;
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::*;
use float_cmp::approx_eq;
use kyogre_core::{CreateFuelMeasurement, TestHelperOutbound};

#[tokio::test]
async fn test_fuel_processor_adds_correct_post_and_pre_estimates() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));

        let end = start + Duration::days(5);

        let fuel_processor = builder.processors.estimator.clone();

        let state = builder
            .trip_data_increment(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .set_engine_building_year()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(60)
            .build()
            .await;

        helper.app.login_user();

        let first_measurement = start + Duration::hours(10);
        let second_measurement = start + Duration::hours(52);

        let body = vec![
            CreateFuelMeasurement {
                timestamp: first_measurement,
                fuel: 3000.,
            },
            CreateFuelMeasurement {
                timestamp: second_measurement,
                fuel: 2000.,
            },
        ];

        helper.app.create_fuel_measurements(&body).await.unwrap();

        fuel_processor.run_single(None).await.unwrap();

        let ranges = helper.adapter().all_fuel_measurement_ranges().await;
        assert_eq!(ranges.len(), 1);

        let range = &ranges[0];

        let expected_pre_ts = first_measurement
            .with_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
            .unwrap();

        let expected_pre = fuel_processor
            .estimate_range(&state.vessels[0], first_measurement, expected_pre_ts)
            .await;

        let expected_post_ts = second_measurement
            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap();

        let expected_post = fuel_processor
            .estimate_range(&state.vessels[0], expected_post_ts, second_measurement)
            .await;

        assert_eq!(
            range.pre_estimate_ts.timestamp(),
            expected_pre_ts.timestamp()
        );
        assert!(approx_eq!(
            f64,
            range.pre_estimate_value.unwrap(),
            expected_pre
        ));

        assert_eq!(
            range.post_estimate_ts.timestamp(),
            expected_post_ts.timestamp()
        );
        assert!(approx_eq!(
            f64,
            range.post_estimate_value.unwrap(),
            expected_post
        ));
    })
    .await;
}
