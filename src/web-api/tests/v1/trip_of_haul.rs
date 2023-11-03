use super::helper::test_with_cache;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::Quality;
use kyogre_core::{HaulId, PrecisionId};
use web_api::routes::v1::trip::Trip;

#[tokio::test]
async fn test_trip_of_haul_returns_none_of_no_trip_is_connected_to_given_haul_id() {
    test_with_cache(|helper, _builder| async move {
        helper.refresh_cache().await;

        let response = helper.app.get_trip_of_haul(&HaulId(7645323266)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_does_not_return_trip_outside_haul_period() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).up().hauls(1).build().await;

        helper.refresh_cache().await;

        let response = helper.app.get_trip_of_haul(&state.hauls[0].haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_does_not_return_trip_of_other_vessels() {
    test_with_cache(|helper, builder| async move {
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .up()
            .vessels(1)
            .hauls(1)
            .modify(|v| {
                v.dca.set_start_timestamp(start + Duration::hours(1));
                v.dca.set_start_timestamp(end - Duration::hours(1));
                v.dca
                    .message_info
                    .set_message_timestamp(start + Duration::hours(1));
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper.app.get_trip_of_haul(&state.hauls[0].haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_all_hauls_and_landings_connected_to_trip() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).hauls(1).build().await;

        helper.refresh_cache().await;

        let response = helper.app.get_trip_of_haul(&state.hauls[0].haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(state.trips[0], body);
    })
    .await;
}

#[tokio::test]
async fn test_aggregates_landing_data_per_product_quality_and_species_id() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .hauls(1)
            .landings(4)
            .modify_idx(|i, v| match i {
                0 | 1 => {
                    v.landing.product.quality = Quality::Prima;
                    v.landing.product.species.fdir_code = 1;
                }
                2 | 3 => {
                    v.landing.product.quality = Quality::A;
                    v.landing.product.species.fdir_code = 2;
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper.app.get_trip_of_haul(&state.hauls[0].haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(state.trips[0], body);
        assert_eq!(body.delivery.delivered.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_precision_range_of_trip_if_it_exists() {
    test_with_cache(|helper, builder| async move {
        let start = Utc.timestamp_opt(1000000, 0).unwrap();
        let end = Utc.timestamp_opt(2000000, 0).unwrap();
        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .precision(PrecisionId::Port)
            .hauls(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper.app.get_trip_of_haul(&state.hauls[0].haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert!(body.start != start);
        assert!(body.end != end);
    })
    .await;
}
