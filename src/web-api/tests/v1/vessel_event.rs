use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use kyogre_core::ScraperInboundPort;
use kyogre_core::{FiskeridirVesselId, VesselEventType};
use web_api::routes::v1::trip::{Trip, TripsParameters};

#[tokio::test]
async fn test_trips_does_not_contain_duplicated_tra_events() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let tra =
            fiskeridir_rs::ErsTra::test_default(1, Some(fiskeridir_vessel_id.0 as u64), start);
        let tra2 = tra.clone();

        helper.adapter().add_ers_tra(vec![tra]).await.unwrap();
        helper.adapter().add_ers_tra(vec![tra2]).await.unwrap();

        let mut ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let mut trips: Vec<Trip> = response.json().await.unwrap();
        trips[0]
            .events
            .retain(|v| matches!(v.event_type, VesselEventType::ErsTra));
        ers_trip
            .vessel_events
            .retain(|v| matches!(v.event_type, VesselEventType::ErsTra));
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 1);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_contain_duplicated_dca_events() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let mut dca = fiskeridir_rs::ErsDca::test_default(1, Some(fiskeridir_vessel_id.0 as u64));
        dca.set_start_timestamp(start + Duration::seconds(1));
        dca.set_stop_timestamp(end - Duration::seconds(1));
        let dca2 = dca.clone();

        helper.adapter().add_ers_dca(vec![dca]).await.unwrap();
        helper.adapter().add_ers_dca(vec![dca2]).await.unwrap();

        let mut ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let mut trips: Vec<Trip> = response.json().await.unwrap();
        trips[0]
            .events
            .retain(|v| matches!(v.event_type, VesselEventType::ErsDca));
        ers_trip
            .vessel_events
            .retain(|v| matches!(v.event_type, VesselEventType::ErsDca));
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 1);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}
