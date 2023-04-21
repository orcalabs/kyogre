use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::{CallSign, Quality};
use kyogre_core::{FiskeridirVesselId, HaulId, Mmsi, ScraperInboundPort};
use web_api::routes::v1::trip::Trip;

#[tokio::test]
async fn test_trip_of_haul_returns_none_of_no_trip_is_connected_to_given_haul_id() {
    test(|helper| async move {
        let response = helper
            .app
            .get_trip_of_haul(&HaulId("non-existing".into()))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_ers_based_trip_in_favour_of_landings_if_both_exist() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;
        helper
            .generate_landings_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(ers_trip, body);
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_landings_based_trip_if_ers_based_does_not_exist() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let landings_trip = helper
            .generate_landings_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();

        assert_eq!(landings_trip, body);
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_does_not_return_trip_outside_haul_period() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let start = Utc.timestamp_opt(1000000, 0).unwrap();
        let end = Utc.timestamp_opt(10000000, 0).unwrap();
        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &(start - Duration::days(4)),
                &(start - Duration::days(3)),
            )
            .await;
        helper
            .generate_landings_trip(
                fiskeridir_vessel_id,
                &(end + Duration::days(3)),
                &(end + Duration::days(4)),
            )
            .await;

        let haul = helper
            .db
            .generate_haul(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_does_not_return_trip_of_other_vessels() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(12);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();
        helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;
        helper
            .generate_landings_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id2,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_all_hauls_and_landings_connected_to_trip() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::hours(1);

        helper
            .db
            .db
            .add_landings(vec![landing], 2023)
            .await
            .unwrap();

        let trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(trip, body);
    })
    .await;
}

#[tokio::test]
async fn test_aggregates_landing_data_per_product_quality_and_species_id() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::hours(1);
        landing.product.quality = Quality::Prima;
        landing.product.species.fdir_code = 1;

        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id.0));
        landing2.landing_timestamp = start + Duration::hours(1);
        landing2.product.quality = Quality::Prima;
        landing2.product.species.fdir_code = 1;

        let mut landing3 = fiskeridir_rs::Landing::test_default(3, Some(fiskeridir_vessel_id.0));
        landing3.landing_timestamp = start + Duration::hours(1);
        landing3.product.quality = Quality::A;
        landing3.product.species.fdir_code = 2;

        let mut landing4 = fiskeridir_rs::Landing::test_default(4, Some(fiskeridir_vessel_id.0));
        landing4.landing_timestamp = start + Duration::hours(1);
        landing4.product.quality = Quality::A;
        landing4.product.species.fdir_code = 2;

        helper
            .db
            .db
            .add_landings(vec![landing.clone(), landing2, landing3, landing4], 2023)
            .await
            .unwrap();

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(trip, body);

        assert_eq!(body.delivery.delivered.len(), 2);
        assert_eq!(
            body.delivered_per_delivery_point[&landing.delivery_point.id.unwrap()]
                .delivered
                .len(),
            2
        );
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_precision_range_of_trip_if_it_exists() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("LK-28").unwrap();
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;
        let vessel = helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, Some(call_sign))
            .await;
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let precision_update = helper.add_precision_to_trip(&vessel, &ers_trip).await;

        let response = helper.app.get_trip_of_haul(&haul.haul_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(precision_update.start(), body.start);
        assert_eq!(precision_update.end(), body.end);
    })
    .await;
}
