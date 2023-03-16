use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use kyogre_core::FiskeridirVesselId;
use web_api::routes::v1::trip::Trip;

#[tokio::test]
async fn test_trip_of_haul_returns_none_of_no_trip_is_connected_to_given_haul_id() {
    test(|helper| async move {
        let response = helper.app.get_trip_of_haul("non-existing").await;
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
        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;
        helper
            .generate_landings_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
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
        let landings_trip = helper
            .generate_landings_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let haul = helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
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
