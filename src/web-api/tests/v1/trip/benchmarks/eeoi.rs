use crate::v1::helper::test;
use engine::*;
use web_api::routes::v1::trip::benchmarks::AverageEeoiParams;

#[tokio::test]
async fn test_eeoi_benchmark_works() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .ais_vms_positions(30)
            .hauls(1)
            .modify(|v| {
                v.dca.catch.species.living_weight = Some(100);
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
        assert!(bench.trips[0].eeoi.unwrap() > 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_eeoi_benchmark_does_not_compute_on_trips_with_low_distances() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .ais_vms_positions(2)
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
        assert!(bench.trips[0].eeoi.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_eeoi_benchmark_does_not_compute_on_trips_with_no_position_data() {
    test(|mut helper, builder| async move {
        builder
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
        assert!(bench.trips[0].eeoi.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_eeoi_benchmark_does_not_compute_without_landings() {
    test(|mut helper, builder| async move {
        builder.vessels(1).set_logged_in().trips(1).build().await;

        helper.app.login_user();

        let bench = helper
            .app
            .get_trip_benchmarks(Default::default())
            .await
            .unwrap();

        assert_eq!(bench.trips.len(), 1);
        assert!(bench.trips[0].eeoi.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_eeoi_works() {
    test(|mut helper, builder| async move {
        builder
            .vessels(1)
            .set_logged_in()
            .trips(1)
            .landings(1)
            .ais_positions(3)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        helper.app.login_user();

        let eeoi = helper.app.get_eeoi(Default::default()).await.unwrap();
        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_average_eeoi_works() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(10)
            .trips(20)
            .landings(20)
            .ais_positions(100)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        let start = state.trips[0].period.start();
        let end = state.trips.last().unwrap().period.end();

        let eeoi = helper
            .app
            .get_average_eeoi(AverageEeoiParams {
                start_date: start,
                end_date: end,
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_average_eeoi_works_on_single_trip() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .ais_positions(10)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        let start = state.trips[0].period.start();
        let end = state.trips.last().unwrap().period.end();

        let eeoi = helper
            .app
            .get_average_eeoi(AverageEeoiParams {
                start_date: start,
                end_date: end,
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_average_eeoi_works_on_two_trips() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(2)
            .trips(2)
            .landings(2)
            .ais_positions(10)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        let start = state.trips[0].period.start();
        let end = state.trips.last().unwrap().period.end();

        let eeoi = helper
            .app
            .get_average_eeoi(AverageEeoiParams {
                start_date: start,
                end_date: end,
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}
