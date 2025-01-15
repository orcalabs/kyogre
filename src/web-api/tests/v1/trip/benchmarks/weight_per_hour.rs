use crate::v1::helper::test;
use chrono::Duration;
use engine::*;

#[tokio::test]
async fn test_weight_per_hour_is_correct() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(2)
            .hauls(2)
            .modify_idx(|i, v| {
                v.dca.catch.species.living_weight = Some((i + 1) as u32 * 1_000);
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
        assert_eq!(bench.trips[0].weight_per_hour.unwrap() as i64, 1000);
        assert_eq!(bench.trips[1].weight_per_hour.unwrap() as i64, 500);
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_hour_sets_trips_with_zero_weight_to_zero() {
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
        assert_eq!(bench.trips[0].weight_per_hour.unwrap(), 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_hour_does_not_include_unrealistic_values_for_landing_assembler() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
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

        assert_eq!(bench.trips.len(), 1);
        assert!(bench.trips[0].weight_per_hour.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_weight_per_hour_does_not_include_unrealistic_values_for_ers_assembler() {
    test(|mut helper, builder| async move {
        builder
            .trip_duration(Duration::hours(2))
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .hauls(1)
            .modify(|h| {
                h.dca.catch.species.living_weight = Some(1_000_000_000);
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
        assert!(bench.trips[0].weight_per_hour.is_none());
    })
    .await;
}
