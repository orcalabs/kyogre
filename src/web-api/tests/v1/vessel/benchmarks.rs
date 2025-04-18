use super::super::helper::test;
use chrono::{Datelike, Duration, TimeZone, Utc};
use engine::*;
use http_client::StatusCode;
use kyogre_core::{Month, TEST_SIGNED_IN_VESSEL_CALLSIGN};

#[tokio::test]
async fn test_vessel_benchmarks_without_token_returns_not_found() {
    test(|helper, _builder| async move {
        let error = helper.app.get_vessel_benchmarks().await.unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_cumulative_landings() {
    test(|mut helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(1)
            .set_logged_in()
            .landings(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(2022, 1, 1, 1, 0, 0).unwrap();
                    v.landing.product.species.fdir_code = 201
                }
                1 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(now.year(), 2, 1, 1, 0, 0).unwrap();
                    v.landing.product.living_weight = Some(200.0);
                    v.landing.product.species.fdir_code = 201
                }
                2 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(now.year(), 3, 1, 1, 0, 0).unwrap();
                    v.landing.product.living_weight = Some(300.0);
                    v.landing.product.species.fdir_code = 201
                }
                3 => {
                    v.landing.landing_timestamp =
                        Utc.with_ymd_and_hms(now.year(), 3, 1, 1, 0, 0).unwrap();
                    v.landing.product.living_weight = Some(5000.0);
                    v.landing.product.species.fdir_code = 200
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        helper.app.login_user();

        let benchmarks = helper.app.get_vessel_benchmarks().await.unwrap();
        assert_eq!(benchmarks.cumulative_landings.len(), 3);
        assert_eq!(benchmarks.cumulative_landings[0].species_fiskeridir_id, 201);
        assert_eq!(benchmarks.cumulative_landings[1].species_fiskeridir_id, 200);
        assert_eq!(benchmarks.cumulative_landings[2].species_fiskeridir_id, 201);

        assert_eq!(benchmarks.cumulative_landings[0].month, Month::February);
        assert_eq!(benchmarks.cumulative_landings[1].month, Month::March);
        assert_eq!(benchmarks.cumulative_landings[2].month, Month::March);

        assert_eq!(benchmarks.cumulative_landings[0].weight as i32, 200);
        assert_eq!(benchmarks.cumulative_landings[1].weight as i32, 5000);
        assert_eq!(benchmarks.cumulative_landings[2].weight as i32, 300);

        assert_eq!(
            benchmarks.cumulative_landings[0].cumulative_weight as i32,
            200
        );
        assert_eq!(
            benchmarks.cumulative_landings[1].cumulative_weight as i32,
            5000
        );
        assert_eq!(
            benchmarks.cumulative_landings[2].cumulative_weight as i32,
            500
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_self_benchmarks() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .set_logged_in()
            .trips(2)
            .hauls(4)
            .landings(4)
            .ais_positions(4)
            .build()
            .await;

        helper.app.login_user();

        let benchmarks = helper.app.get_vessel_benchmarks().await.unwrap();

        let fishing_distance = benchmarks.fishing_distance.unwrap();
        let fishing_time = benchmarks.fishing_time.unwrap();
        let trip_time = benchmarks.trip_time.unwrap();
        let landings = benchmarks.landings.unwrap();
        let ers_dca = benchmarks.ers_dca.unwrap();

        // All hauls in test have same duration
        let fishing_time_per_trip = (state.hauls[0].duration().num_minutes() * 2) as f64;
        let trip_time_minutes =
            (state.trips[0].period.end() - state.trips[0].period.start()).num_minutes() as f64;
        // All landings in test have same weight
        let landing_weight_per_trip = state.landings[0].total_living_weight * 2.0;
        // All hauls in test have same weight
        let haul_weight_per_trip = state.hauls[0].total_living_weight() as f64 * 2.0;

        // Fishing time
        assert_eq!(
            fishing_time.recent_trips[0],
            (&state.trips[0], fishing_time_per_trip),
        );
        assert_eq!(
            fishing_time.recent_trips[1],
            (&state.trips[1], fishing_time_per_trip),
        );
        assert_eq!(fishing_time.average_followers, 0.0);
        assert_eq!(fishing_time.average, fishing_time_per_trip);
        // Fishing distance
        assert_eq!(fishing_distance.recent_trips[0], (&state.trips[0], 116.0));
        assert_eq!(fishing_distance.recent_trips[1], (&state.trips[1], 116.0));
        assert_eq!(fishing_distance.average_followers, 0.0);
        assert_eq!(fishing_distance.average as i64, 116);
        // Trip time
        assert_eq!(
            trip_time.recent_trips[0],
            (&state.trips[0], trip_time_minutes),
        );
        assert_eq!(
            trip_time.recent_trips[1],
            (&state.trips[1], trip_time_minutes),
        );
        assert_eq!(trip_time.average_followers, 0.0);
        assert_eq!(trip_time.average, trip_time_minutes);
        // Landings
        assert_eq!(
            landings.recent_trips[0],
            (&state.trips[0], landing_weight_per_trip),
        );
        assert_eq!(
            landings.recent_trips[1],
            (&state.trips[1], landing_weight_per_trip),
        );
        assert_eq!(landings.average_followers, 0.0);
        assert_eq!(landings.average, landing_weight_per_trip);
        // Ers dca
        assert_eq!(
            ers_dca.recent_trips[0],
            (&state.trips[0], haul_weight_per_trip),
        );
        assert_eq!(
            ers_dca.recent_trips[1],
            (&state.trips[1], haul_weight_per_trip),
        );
        assert_eq!(ers_dca.average_followers, 0.0);
        assert_eq!(ers_dca.average, haul_weight_per_trip);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_excludes_data_from_non_active_vessels() {
    test(|mut helper, builder| async move {
        let start = Utc::now() - Duration::days(20);

        let end = start + Duration::days(10);

        let state = builder
            .trip_data_increment(Duration::hours(6))
            .vessels(1)
            .set_engine_building_year()
            .set_logged_in()
            .active_vessel()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .up()
            .vessels(1)
            .set_engine_building_year()
            .set_call_sign(&(TEST_SIGNED_IN_VESSEL_CALLSIGN.try_into().unwrap()))
            .historic_vessel()
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .ais_vms_positions(40)
            .build()
            .await;

        helper.app.login_user();

        helper.builder().await.build().await;

        let benchmarks = helper.app.get_vessel_benchmarks().await.unwrap();

        assert_eq!(benchmarks.trip_time.as_ref().unwrap().recent_trips.len(), 1);
        assert_eq!(
            benchmarks.trip_time.unwrap().recent_trips[0].fiskeridir_vessel_id,
            state.vessels[0].fiskeridir.id
        );
    })
    .await;
}
