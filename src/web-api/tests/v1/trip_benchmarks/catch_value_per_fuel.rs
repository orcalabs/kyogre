use chrono::{Duration, TimeZone, Utc};
use engine::{Modifiable, TripLevel};

use crate::v1::helper::test;

#[tokio::test]
async fn test_catch_value_per_fuel_is_correct() {
    test(|mut helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(100000000000, 0).unwrap();

        let speed = 5.;

        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .landings(1)
            .modify(|v| {
                v.landing.finances.price_for_fisher = Some(1_000.);
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
        assert!(bench.trips[0].catch_value_per_fuel.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_catch_value_per_fuel_does_not_compute_trips_without_price_for_fisher() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .modify(|v| {
                v.landing.finances.price_for_fisher = None;
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
        assert!(bench.trips[0].catch_value_per_fuel.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_catch_value_per_fuel_does_not_compute_trips_with_zero_fuel() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .modify(|v| {
                v.landing.finances.price_for_fisher = Some(1_000.);
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
        assert!(bench.trips[0].catch_value_per_fuel.is_none());
    })
    .await;
}
