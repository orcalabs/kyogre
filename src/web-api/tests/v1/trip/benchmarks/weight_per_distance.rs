use chrono::{Duration, TimeZone, Utc};
use engine::{Modifiable, TripLevel};

use crate::v1::helper::test;

#[tokio::test]
async fn test_weight_per_distance_is_correct() {
    test(|mut helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(100000000000, 0).unwrap();
        let weight = 1_000.;

        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .hauls(1)
            .modify(|v| {
                v.dca.catch.species.living_weight = Some(weight as _);
            })
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.msgtime = start + Duration::seconds(100000);
                    v.position.latitude = 13.5;
                    v.position.longitude = 67.5;
                }
                1 => {
                    v.position.msgtime = start + Duration::seconds(200000);
                    v.position.latitude = 14.5;
                    v.position.longitude = 68.5;
                }
                2 => {
                    v.position.msgtime = start + Duration::seconds(300000);
                    v.position.latitude = 15.5;
                    v.position.longitude = 69.5;
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

        // Verified to be correct using https://www.nhc.noaa.gov/gccalc.shtml
        let distance = 308939.644;

        assert_eq!(bench.trips.len(), 1);
        assert_eq!(
            bench.trips[0].weight_per_distance.unwrap() as i64,
            (weight / distance) as i64
        );
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_distance_sets_trips_with_zero_weight_to_zero() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .modify(|l| {
                l.landing.product.living_weight = Some(0.);
            })
            .ais_positions(3)
            .build()
            .await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        assert_eq!(bench.trips.len(), 1);
        assert_eq!(bench.trips[0].weight_per_distance.unwrap(), 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_distance_does_not_compute_trips_with_zero_distance() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .modify(|l| {
                l.landing.product.living_weight = Some(0.);
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
        assert!(bench.trips[0].weight_per_distance.is_none());
    })
    .await;
}
