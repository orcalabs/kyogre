use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use engine::{Modifiable, TripLevel};
use kyogre_core::OptionalDateTimeRange;
use web_api::routes::v1::trip::benchmarks::TripBenchmarksParams;

use crate::v1::helper::test;

#[tokio::test]
async fn test_fuel_consumption_is_correct() {
    test(|mut helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(100000000000, 0).unwrap();

        let speed = 5.;
        let engine_power = 10.;

        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.engine_power = Some(engine_power as _);
            })
            .set_logged_in()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .landings(1)
            .modify(|v| {
                v.landing.vessel.engine_building_year = Some(2000);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.msgtime = start + Duration::seconds(100000);
                    v.position.latitude = 13.5;
                    v.position.longitude = 67.5;
                    v.position.speed_over_ground = Some(speed);
                }
                1 => {
                    v.position.msgtime = start + Duration::seconds(200000);
                    v.position.latitude = 14.5;
                    v.position.longitude = 68.5;
                    v.position.speed_over_ground = Some(speed);
                }
                2 => {
                    v.position.msgtime = start + Duration::seconds(300000);
                    v.position.latitude = 15.5;
                    v.position.longitude = 69.5;
                    v.position.speed_over_ground = Some(speed);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        assert_eq!(bench.trips.len(), 1);
        assert!(bench.trips[0].fuel_consumption.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_detailed_trip_includes_fuel_consumption() {
    test(|mut helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(100000000000, 0).unwrap();

        let speed = 5.;
        let engine_power = 10.;

        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.engine_power = Some(engine_power as _);
            })
            .set_logged_in()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .landings(1)
            .modify(|v| {
                v.landing.vessel.engine_building_year = Some(2000);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.msgtime = start + Duration::seconds(100000);
                    v.position.latitude = 13.5;
                    v.position.longitude = 67.5;
                    v.position.speed_over_ground = Some(speed);
                }
                1 => {
                    v.position.msgtime = start + Duration::seconds(200000);
                    v.position.latitude = 14.5;
                    v.position.longitude = 68.5;
                    v.position.speed_over_ground = Some(speed);
                }
                2 => {
                    v.position.msgtime = start + Duration::seconds(300000);
                    v.position.latitude = 15.5;
                    v.position.longitude = 69.5;
                    v.position.speed_over_ground = Some(speed);
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        helper.app.login_user();

        let trips = helper.app.get_trips(Default::default()).await.unwrap();

        assert_eq!(trips.len(), 1);
        assert!(trips[0].fuel_consumption.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_fuel_consumption_does_not_compute_trips_with_zero_distance() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .build()
            .await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        assert_eq!(bench.trips.len(), 1);
        assert!(bench.trips[0].fuel_consumption.is_none());
    })
    .await;
}
#[tokio::test]
async fn test_fuel_consumption_handles_positions_with_equal_timestamp() {
    test(|mut helper, builder| async move {
        let start = Utc.from_utc_datetime(&NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 3, 12).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ));
        let end = start + Duration::days(10);

        builder
            .vessels(1)
            .set_logged_in()
            .set_engine_building_year()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(10)
            .modify_idx(|i, v| {
                if i <= 1 {
                    v.position.set_timestamp(start + Duration::hours(1));
                    v.position.set_location(72.12, 25.12);
                } else {
                    v.position.set_timestamp(start + Duration::hours(i as i64));
                }
            })
            .build()
            .await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(TripBenchmarksParams {
                range: OptionalDateTimeRange::test_new(Some(start), Some(end)),
                ordering: None,
            })
            .await
            .unwrap();

        assert_eq!(bench.trips.len(), 1);
        assert!(bench.trips[0].fuel_consumption.unwrap() > 0.);
    })
    .await;
}
