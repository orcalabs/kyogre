use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::Gear;
use kyogre_core::{DatabaseViewRefresher, ScraperInboundPort};
use kyogre_core::{FiskeridirVesselId, VesselEventType};
use web_api::routes::v1::trip::{Trip, TripsParameters};

#[tokio::test]
async fn test_trips_does_not_contain_duplicated_tra_events() {
    test(|mut helper, _builder| async move {
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
    test(|mut helper, _builder| async move {
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

        helper.db.add_ers_dca_value(dca).await;
        helper.db.add_ers_dca_value(dca2).await;

        let mut ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let mut trips: Vec<Trip> = response.json().await.unwrap();
        trips[0]
            .events
            .retain(|v| matches!(v.event_type, VesselEventType::Haul));
        ers_trip
            .vessel_events
            .retain(|v| matches!(v.event_type, VesselEventType::Haul));
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 1);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::Haul);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_events_connect_to_existing_trip() {
    test(|mut helper, _builder| async move {
        let vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper
            .db
            .generate_haul(
                vessel_id,
                &(start + Duration::seconds(1)),
                &(end - Duration::seconds(1)),
            )
            .await;

        let tra = fiskeridir_rs::ErsTra::test_default(
            1,
            Some(vessel_id.0 as u64),
            start + Duration::seconds(2),
        );
        helper.adapter().add_ers_tra(vec![tra]).await.unwrap();

        helper
            .db
            .generate_landing(1, vessel_id, start + Duration::seconds(3))
            .await;

        let mut dca = fiskeridir_rs::ErsDca::test_default(10, Some(vessel_id.0 as u64));
        let d = start + Duration::seconds(4);
        dca.message_info.message_date = d.date_naive();
        dca.message_info.message_time = d.time();
        dca.gear.gear_fdir_code = Gear::Ukjent;
        dca.catch.species.species_fao_code = None;
        dca.catch.species.living_weight = None;
        dca.whale_catch_info.grenade_number = None;
        helper.db.add_ers_dca_value(dca).await;

        helper.adapter().refresh().await.unwrap();

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 7);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::Haul);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::Landing);
        assert_eq!(trips[0].events[5].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[6].event_type, VesselEventType::ErsPor);
    })
    .await;
}
