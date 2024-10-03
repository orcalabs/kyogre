use chrono::Duration;
use engine::{Modifiable, TripLevel};

use crate::v1::helper::test;

#[tokio::test]
async fn test_weight_per_hour_is_correct() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(2)
            .landings(2)
            .modify_idx(|i, l| {
                l.landing.product.living_weight = Some((i + 1) as f64 * 1_000.);
            })
            .build()
            .await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        assert_eq!(bench.trips.len(), 2);
        assert_eq!(bench.trips[0].weight_per_hour as i64, 1000);
        assert_eq!(bench.trips[1].weight_per_hour as i64, 500);
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_hour_does_not_compute_trips_with_zero_weight() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
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

        assert!(bench.trips.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_hour_does_not_include_unrealistic_values() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .trips(1)
            .landings(1)
            .modify(|l| {
                l.landing.product.living_weight = Some(1_000_000_000.);
            })
            .build()
            .await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        assert!(bench.trips.is_empty());
    })
    .await;
}
