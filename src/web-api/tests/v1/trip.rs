use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::{CallSign, ErsDep, ErsPor, Quality};
use kyogre_core::{
    FiskeridirVesselId, HaulId, Mmsi, Ordering, ScraperInboundPort, VesselEventType,
};
use web_api::routes::v1::trip::{CurrentTrip, Trip, TripsParameters};

#[tokio::test]
async fn test_trip_of_haul_returns_none_of_no_trip_is_connected_to_given_haul_id() {
    test(|helper| async move {
        let response = helper.app.get_trip_of_haul(&HaulId(7645323266)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
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
    test(|mut helper| async move {
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
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(12);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();
        helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
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
    test(|mut helper| async move {
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
    test(|mut helper| async move {
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
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_haul_returns_precision_range_of_trip_if_it_exists() {
    test(|mut helper| async move {
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

#[tokio::test]
async fn test_trips_of_vessel_only_returns_trips_of_specified_vessel() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(2);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id2, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        helper
            .generate_ers_trip(fiskeridir_vessel_id2, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_filters_by_limit() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let start2 = Utc.timestamp_opt(10000000, 0).unwrap();
        let end2 = Utc.timestamp_opt(200000000, 0).unwrap();

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        helper
            .generate_ers_trip(fiskeridir_vessel_id, &start2, &end2)
            .await;

        let params = TripsParameters {
            limit: Some(1),
            offset: None,
            ordering: Some(Ordering::Asc),
        };

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, params, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_filters_by_offset() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let start2 = Utc.timestamp_opt(10000000, 0).unwrap();
        let end2 = Utc.timestamp_opt(200000000, 0).unwrap();

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        helper
            .generate_ers_trip(fiskeridir_vessel_id, &start2, &end2)
            .await;

        let params = TripsParameters {
            limit: None,
            offset: Some(1),
            ordering: Some(Ordering::Desc),
        };

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, params, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_orders_by_period() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let start2 = Utc.timestamp_opt(10000000, 0).unwrap();
        let end2 = Utc.timestamp_opt(200000000, 0).unwrap();

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let ers_trip2 = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start2, &end2)
            .await;

        let params = TripsParameters {
            limit: None,
            offset: None,
            ordering: Some(Ordering::Asc),
        };

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, params, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], ers_trip);
        assert_eq!(trips[1], ers_trip2);
    })
    .await;
}

#[tokio::test]
async fn test_first_ers_data_triggers_trip_assembler_switch_to_ers() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let landings_trip = helper
            .generate_landings_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        // Landings generates two trips on the first landing
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[1], landings_trip);

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_all_events_within_trip_period_ordered_ascendingly() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper
            .db
            .generate_landing(1, fiskeridir_vessel_id, start + Duration::seconds(1))
            .await;

        helper
            .db
            .generate_tra(1, fiskeridir_vessel_id, start + Duration::seconds(2))
            .await;

        helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::seconds(3)),
                &(end - Duration::seconds(3)),
            )
            .await;

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 5);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::Landing);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_events_are_isolated_per_vessel() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(2);
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id2, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper
            .db
            .generate_landing(1, fiskeridir_vessel_id, start + Duration::seconds(1))
            .await;

        helper
            .db
            .generate_tra(1, fiskeridir_vessel_id, start + Duration::seconds(1))
            .await;

        helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::seconds(1)),
                &(end - Duration::seconds(1)),
            )
            .await;

        helper
            .db
            .generate_landing(2, fiskeridir_vessel_id2, start + Duration::seconds(1))
            .await;

        helper
            .db
            .generate_tra(2, fiskeridir_vessel_id2, start + Duration::seconds(1))
            .await;

        helper
            .db
            .generate_haul(
                fiskeridir_vessel_id2,
                &(start + Duration::seconds(1)),
                &(end - Duration::seconds(1)),
            )
            .await;

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let ers_trip2 = helper
            .generate_ers_trip(fiskeridir_vessel_id2, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 5);
        assert_eq!(trips[0], ers_trip);

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id2, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 5);
        assert_eq!(trips[0], ers_trip2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_include_events_outside_period() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(2);
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id2, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper
            .db
            .generate_tra(1, fiskeridir_vessel_id, end + Duration::seconds(1))
            .await;

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 2);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_tra_events_without_timestamps() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(2);
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id2, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper
            .db
            .generate_tra(1, fiskeridir_vessel_id, end + Duration::seconds(1))
            .await;

        let mut tra =
            fiskeridir_rs::ErsTra::test_default(0, Some(fiskeridir_vessel_id.0 as u64), start);
        tra.reloading_timestamp = None;
        tra.reloading_date = None;
        tra.reloading_time = None;
        helper.db.db.add_ers_tra(vec![tra]).await.unwrap();

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(fiskeridir_vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);

        assert_eq!(trips[0].events.len(), 2);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0], ers_trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_returns_correct_ports() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        let start_port = "NOTOS".to_string();
        let end_port = "DENOR".to_string();
        let mut departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        departure.port.code = Some(start_port.clone());
        let mut arrival = ErsPor::test_default(1, vessel_id.0 as u64, end, 2);
        arrival.port.code = Some(end_port.clone());

        let ers_trip = helper
            .generate_ers_trip_with_messages(vessel_id, departure, arrival)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], ers_trip);
        assert_eq!(trips[0].start_port_id.clone().unwrap(), start_port);
        assert_eq!(trips[0].end_port_id.clone().unwrap(), end_port);
    })
    .await;
}

#[tokio::test]
async fn test_trip_contains_correct_arrival_and_departure_with_adjacent_trips_with_equal_start_and_stop(
) {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();
        let end2 = Utc.timestamp_opt(300000, 1).unwrap();
        let end3 = Utc.timestamp_opt(400000, 1).unwrap();

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        let departure2 = ErsDep::test_default(2, vessel_id.0 as u64, end, 3);
        let arrival2 = ErsPor::test_default(2, vessel_id.0 as u64, end2, 4);

        let departure3 = ErsDep::test_default(3, vessel_id.0 as u64, end2, 4);
        let arrival3 = ErsPor::test_default(3, vessel_id.0 as u64, end3, 5);

        helper
            .generate_ers_trip_with_messages(vessel_id, departure, arrival)
            .await;

        helper
            .generate_ers_trip_with_messages(vessel_id, departure2, arrival2)
            .await;

        helper
            .generate_ers_trip_with_messages(vessel_id, departure3, arrival3)
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 3);
        assert_eq!(trips[0].events.len(), 2);
        assert_eq!(trips[1].events.len(), 2);
        assert_eq!(trips[2].events.len(), 2);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[1].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[1].events[1].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[2].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[2].events[1].event_type, VesselEventType::ErsPor);
    })
    .await;
}

#[tokio::test]
async fn test_ers_trip_contains_events_added_after_trip_creation() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper
            .db
            .generate_landing(1, vessel_id, start + Duration::seconds(1))
            .await;
        helper
            .db
            .generate_tra(1, vessel_id, start + Duration::seconds(2))
            .await;

        helper
            .db
            .generate_haul(
                vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips[0].events.len(), 5);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::Landing);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::ErsPor);
    })
    .await;
}

#[tokio::test]
async fn test_landings_trip_contains_events_added_after_trip_creation() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(1000000000, 0).unwrap();

        helper.generate_landings_trip(vessel_id, &start, &end).await;

        helper
            .db
            .generate_ers_arrival_with_port(1, vessel_id, start + Duration::seconds(1), 1, "NOTOS")
            .await;
        helper
            .db
            .generate_tra(1, vessel_id, start + Duration::seconds(2))
            .await;

        helper
            .db
            .generate_haul(
                vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[1].events.len(), 4);
        assert_eq!(trips[1].events[0].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[1].events[1].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[1].events[2].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[1].events[3].event_type, VesselEventType::Landing);
    })
    .await;
}

#[tokio::test]
async fn test_trip_contains_fishing_facilities() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let setup = Utc.timestamp_opt(200000, 1).unwrap();
        let removed = Utc.timestamp_opt(300000, 1).unwrap();
        let end = Utc.timestamp_opt(400000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = setup;
        facility1.removed_timestamp = Some(removed);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = setup;
        facility2.removed_timestamp = Some(removed);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper
            .generate_ers_trip_with_messages(vessel_id, departure, arrival)
            .await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_trip_does_not_return_fishing_facilities_without_token() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let setup = Utc.timestamp_opt(200000, 1).unwrap();
        let removed = Utc.timestamp_opt(300000, 1).unwrap();
        let end = Utc.timestamp_opt(400000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = setup;
        facility1.removed_timestamp = Some(removed);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = setup;
        facility2.removed_timestamp = Some(removed);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper
            .generate_ers_trip_with_messages(vessel_id, departure, arrival)
            .await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 2);

        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_trip_does_not_return_fishing_facilities_without_read_fishing_facility() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let setup = Utc.timestamp_opt(200000, 1).unwrap();
        let removed = Utc.timestamp_opt(300000, 1).unwrap();
        let end = Utc.timestamp_opt(400000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = setup;
        facility1.removed_timestamp = Some(removed);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = setup;
        facility2.removed_timestamp = Some(removed);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper
            .generate_ers_trip_with_messages(vessel_id, departure, arrival)
            .await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 2);

        let token = helper.bw_helper.get_bw_token_with_policies(vec![]);
        let response = helper
            .app
            .get_trips_of_vessel(vessel_id, TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_current_trip_without_prior_trip() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_current_trip(vessel_id, Some(token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_current_trip_with_prior_trips() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start1 = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();
        let end1 = Utc.timestamp_opt(400000, 1).unwrap();

        let start2 = Utc.timestamp_opt(500000, 1).unwrap();
        let middle3 = Utc.timestamp_opt(600000, 1).unwrap();
        let middle4 = Utc.timestamp_opt(700000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        helper.generate_ers_trip(vessel_id, &start1, &end1).await;

        let mut facility3 = kyogre_core::FishingFacility::test_default();
        let mut facility4 = kyogre_core::FishingFacility::test_default();
        facility3.call_sign = Some(call_sign.clone());
        facility3.setup_timestamp = middle3;
        facility3.removed_timestamp = Some(middle4);
        facility4.call_sign = Some(call_sign.clone());
        facility4.setup_timestamp = middle3;
        facility4.removed_timestamp = Some(middle4);

        helper
            .db
            .add_fishing_facilities(vec![facility3, facility4])
            .await;

        helper.db.generate_haul(vessel_id, &middle3, &middle3).await;
        helper.db.generate_haul(vessel_id, &middle4, &middle4).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start2, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_current_trip(vessel_id, Some(token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start2.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_null_when_no_current_trip() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        let response = helper.app.get_current_trip(vessel_id, None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: Option<CurrentTrip> = response.json().await.unwrap();

        assert!(trip.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_does_not_include_fishing_facilities_without_token() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let response = helper.app.get_current_trip(vessel_id, None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_does_not_include_fishing_facilities_without_permission() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let token = helper.bw_helper.get_bw_token_with_policies(vec![]);
        let response = helper.app.get_current_trip(vessel_id, Some(token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_earliest_departure_since_previous_trip() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start1 = Utc.timestamp_opt(100000, 1).unwrap();
        let end1 = Utc.timestamp_opt(200000, 1).unwrap();
        let start2 = Utc.timestamp_opt(300000, 1).unwrap();
        let start3 = Utc.timestamp_opt(400000, 1).unwrap();

        helper.generate_ers_trip(vessel_id, &start1, &end1).await;

        let departure1 = ErsDep::test_default(1, vessel_id.0 as u64, start3, 1);
        let departure2 = ErsDep::test_default(2, vessel_id.0 as u64, start2, 2);
        helper
            .db
            .db
            .add_ers_dep(vec![departure2, departure1])
            .await
            .unwrap();

        let response = helper.app.get_current_trip(vessel_id, None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start2.timestamp_millis());
    })
    .await;
}
