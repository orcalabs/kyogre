use crate::v1::helper::test;
use engine::*;
use fiskeridir_rs::SpeciesGroup;
use float_cmp::approx_eq;
use kyogre_core::DateTimeRange;
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
                range: DateTimeRange::test_new(start, end),
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
                range: DateTimeRange::test_new(start, end),
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
                range: DateTimeRange::test_new(start, end),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(eeoi.unwrap() > 0.);
    })
    .await;
}

#[tokio::test]
async fn test_average_eeoi_filters_on_species_group_id_includes_all_quantum_from_trips() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .modify(|l| {
                l.landing.product.living_weight = Some(20.0);
                l.landing.product.species.group_code = SpeciesGroup::Saithe;
            })
            .landings(2)
            .modify(|l| {
                l.landing.product.living_weight = Some(15.0);
                l.landing.product.species.group_code = SpeciesGroup::Mackerel;
            })
            .ais_positions(10)
            .modify_idx(|i, v| {
                v.position.latitude += i as f64;
                v.position.longitude += i as f64;
            })
            .build()
            .await;

        let start = state.trips[0].period.start();
        let end = state.trips.last().unwrap().period.end();

        let eeoi_all_species = helper
            .app
            .get_average_eeoi(AverageEeoiParams {
                range: DateTimeRange::test_new(start, end),
                ..Default::default()
            })
            .await
            .unwrap()
            .unwrap();

        let eeoi_single_species = helper
            .app
            .get_average_eeoi(AverageEeoiParams {
                range: DateTimeRange::test_new(start, end),
                species_group_id: Some(SpeciesGroup::Mackerel),
                ..Default::default()
            })
            .await
            .unwrap()
            .unwrap();

        assert!(eeoi_all_species > 0.);
        assert!(eeoi_single_species > 0.);
        assert!(approx_eq!(f64, eeoi_all_species, eeoi_single_species));
    })
    .await;
}

#[tokio::test]
async fn test_average_eeoi_filters_on_species_group_id_returns_none_if_trip_contains_more_of_other_species()
 {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .modify(|l| {
                l.landing.product.living_weight = Some(20.0);
                l.landing.product.species.group_code = SpeciesGroup::Saithe;
            })
            .landings(2)
            .modify(|l| {
                l.landing.product.living_weight = Some(15.0);
                l.landing.product.species.group_code = SpeciesGroup::Mackerel;
            })
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
                range: DateTimeRange::test_new(start, end),
                species_group_id: Some(SpeciesGroup::Saithe),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(eeoi.is_none());
    })
    .await;
}
