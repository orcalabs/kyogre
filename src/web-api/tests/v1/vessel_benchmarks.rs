use super::{barentswatch_helper::SIGNED_IN_VESSEL_CALLSIGN, helper::test};
use actix_web::http::StatusCode;
use chrono::{Datelike, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::CallSign;
use kyogre_core::VesselBenchmarks;
use web_api::routes::v1::user::User;

#[tokio::test]
async fn test_vessel_benchmarks_without_token_returns_not_found() {
    test(|helper, _builder| async move {
        let response = helper.app.get_vessel_benchmarks(None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_cumulative_landings() {
    test(|helper, builder| async move {
        let now = Utc::now();
        builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.radio_call_sign =
                    Some(CallSign::try_from(SIGNED_IN_VESSEL_CALLSIGN).unwrap())
            })
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

        let response = helper
            .app
            .get_vessel_benchmarks(Some(helper.bw_helper.get_bw_token()))
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: VesselBenchmarks = response.json().await.unwrap();

        assert_eq!(body.cumulative_landings.len(), 3);
        assert_eq!(body.cumulative_landings[0].species_fiskeridir_id, 201);
        assert_eq!(body.cumulative_landings[1].species_fiskeridir_id, 200);
        assert_eq!(body.cumulative_landings[2].species_fiskeridir_id, 201);

        assert_eq!(body.cumulative_landings[0].month, chrono::Month::February);
        assert_eq!(body.cumulative_landings[1].month, chrono::Month::March);
        assert_eq!(body.cumulative_landings[2].month, chrono::Month::March);

        assert_eq!(body.cumulative_landings[0].weight as i32, 200);
        assert_eq!(body.cumulative_landings[1].weight as i32, 5000);
        assert_eq!(body.cumulative_landings[2].weight as i32, 300);

        assert_eq!(body.cumulative_landings[0].cumulative_weight as i32, 200);
        assert_eq!(body.cumulative_landings[1].cumulative_weight as i32, 5000);
        assert_eq!(body.cumulative_landings[2].cumulative_weight as i32, 500);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_self_benchmarks() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .modify(|v| {
                let cs = CallSign::try_from(SIGNED_IN_VESSEL_CALLSIGN).unwrap();
                v.fiskeridir.radio_call_sign = Some(cs.clone());
                v.ais.call_sign = Some(cs);
            })
            .trips(2)
            .hauls(4)
            .landings(4)
            .ais_positions(4)
            .build()
            .await;

        let response = helper
            .app
            .get_vessel_benchmarks(Some(helper.bw_helper.get_bw_token()))
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: VesselBenchmarks = response.json().await.unwrap();

        let fishing_distance = body.fishing_distance.unwrap();
        let fishing_time = body.fishing_time.unwrap();
        let trip_time = body.trip_time.unwrap();
        let landings = body.landings.unwrap();
        let ers_dca = body.ers_dca.unwrap();

        // All hauls in test have same duration
        let fishing_time_per_trip = (state.hauls[0].duration * 2) as f64;
        let trip_time_minutes =
            (state.trips[0].period.end() - state.trips[0].period.start()).num_minutes() as f64;
        // All landings in test have same weight
        let landing_weight_per_trip = state.landings[0].total_living_weight * 2.0;
        // All hauls in test have same weight
        let haul_weight_per_trip = state.hauls[0].total_living_weight as f64 * 2.0;

        // Fishing time
        assert_eq!(
            &fishing_time.recent_trips[0],
            (&state.trips[0], fishing_time_per_trip),
        );
        assert_eq!(
            &fishing_time.recent_trips[1],
            (&state.trips[1], fishing_time_per_trip),
        );
        assert_eq!(fishing_time.average_followers, 0.0);
        assert_eq!(fishing_time.average, fishing_time_per_trip);
        // Fishing distance
        assert_eq!(&fishing_distance.recent_trips[0], (&state.trips[0], 116.0));
        assert_eq!(&fishing_distance.recent_trips[1], (&state.trips[1], 116.0));
        assert_eq!(fishing_distance.average_followers, 0.0);
        assert_eq!(fishing_distance.average as i64, 116);
        // Trip time
        assert_eq!(
            &trip_time.recent_trips[0],
            (&state.trips[0], trip_time_minutes),
        );
        assert_eq!(
            &trip_time.recent_trips[1],
            (&state.trips[1], trip_time_minutes),
        );
        assert_eq!(trip_time.average_followers, 0.0);
        assert_eq!(trip_time.average, trip_time_minutes);
        // Landings
        assert_eq!(
            &landings.recent_trips[0],
            (&state.trips[0], landing_weight_per_trip),
        );
        assert_eq!(
            &landings.recent_trips[1],
            (&state.trips[1], landing_weight_per_trip),
        );
        assert_eq!(landings.average_followers, 0.0);
        assert_eq!(landings.average, landing_weight_per_trip);
        // Ers dca
        assert_eq!(
            &ers_dca.recent_trips[0],
            (&state.trips[0], haul_weight_per_trip),
        );
        assert_eq!(
            &ers_dca.recent_trips[1],
            (&state.trips[1], haul_weight_per_trip),
        );
        assert_eq!(ers_dca.average_followers, 0.0);
        assert_eq!(ers_dca.average, haul_weight_per_trip);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_benchmarks_returns_correct_averages_for_followers() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(3)
            .modify_idx(|i, v| {
                if i == 0 {
                    let cs = CallSign::try_from(SIGNED_IN_VESSEL_CALLSIGN).unwrap();
                    v.fiskeridir.radio_call_sign = Some(cs.clone());
                    v.ais.call_sign = Some(cs);
                }
            })
            .trips(3)
            .hauls(6)
            .landings(6)
            .ais_positions(6)
            .build()
            .await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .update_user(
                User {
                    following: vec![
                        state.vessels[1].fiskeridir.id,
                        state.vessels[2].fiskeridir.id,
                    ],
                },
                token.clone(),
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = helper.app.get_vessel_benchmarks(Some(token)).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: VesselBenchmarks = response.json().await.unwrap();

        let fishing_distance = body.fishing_distance.unwrap();
        let fishing_time = body.fishing_time.unwrap();
        let trip_time = body.trip_time.unwrap();
        let landings = body.landings.unwrap();
        let ers_dca = body.ers_dca.unwrap();

        assert_eq!(
            fishing_time.average_followers as i32,
            state.hauls[0].duration * 2
        );
        assert_eq!(fishing_distance.average_followers as i64, 116);
        assert_eq!(
            trip_time.average_followers as i64,
            (state.trips[0].period.end() - state.trips[0].period.start()).num_minutes()
        );
        assert_eq!(
            landings.average_followers as i64,
            (state.landings[0].total_living_weight * 2.0) as i64
        );
        assert_eq!(
            ers_dca.average_followers as i64,
            state.hauls[0].total_living_weight * 2
        );
    })
    .await;
}