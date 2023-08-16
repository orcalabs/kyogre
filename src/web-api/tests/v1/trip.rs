use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::{
    CallSign, DeliveryPoint, DeliveryPointId, ErsDep, ErsPor, GearGroup, Quality, SpeciesGroup,
    VesselLengthGroup,
};
use kyogre_core::{
    FiskeridirVesselId, HaulId, Mmsi, Ordering, ScraperInboundPort, TripSorting, VesselEventType,
};
use web_api::routes::utils::{self, GearGroupId, SpeciesGroupId};
use web_api::routes::v1::trip::{Trip, TripsOfVesselParameters, TripsParameters};

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

        helper.db.add_landings(vec![landing]).await;

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
async fn test_trip_of_landing_returns_none_of_no_trip_is_connected_to_given_landing_id() {
    test(|helper| async move {
        let response = helper
            .app
            .get_trip_of_landing(&"1-7-0".try_into().unwrap())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_does_not_return_trip_outside_landing_timestamp() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let start = Utc.timestamp_opt(1000000, 0).unwrap();

        helper
            .generate_landings_trip(
                fiskeridir_vessel_id,
                &(start - Duration::days(4)),
                &(start - Duration::days(3)),
            )
            .await;

        let landing = helper
            .db
            .generate_landing(11, fiskeridir_vessel_id, start)
            .await;

        let response = helper.app.get_trip_of_landing(&landing.landing_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_does_not_return_trip_of_other_vessels() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(12);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();
        helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let landing = helper
            .db
            .generate_landing(10, fiskeridir_vessel_id2, start + Duration::hours(1))
            .await;

        let response = helper.app.get_trip_of_landing(&landing.landing_id).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_returns_all_hauls_and_landings_connected_to_trip() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        helper
            .db
            .generate_haul(
                fiskeridir_vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::hours(1);

        helper.db.add_landings(vec![landing.clone()]).await;

        let trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trip_of_landing(&landing.id).await;
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
            .add_landings(vec![landing.clone(), landing2, landing3, landing4])
            .await;

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
            .get_trips_of_vessel(
                fiskeridir_vessel_id,
                TripsOfVesselParameters::default(),
                None,
            )
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

        let params = TripsOfVesselParameters {
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

        let params = TripsOfVesselParameters {
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

        let params = TripsOfVesselParameters {
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

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        // Landings generates two trips on the first landing
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], landings_trip);

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
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

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::Landing);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::Haul);
        assert_eq!(trips[0].events[5].event_type, VesselEventType::ErsPor);
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
            .generate_landings(vec![
                (1, fiskeridir_vessel_id, start + Duration::seconds(1)),
                (2, fiskeridir_vessel_id2, start + Duration::seconds(1)),
            ])
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
            .get_trips(
                TripsParameters {
                    fiskeridir_vessel_ids: Some(vec![fiskeridir_vessel_id]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0], ers_trip);

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    fiskeridir_vessel_ids: Some(vec![fiskeridir_vessel_id2]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0], ers_trip2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_include_events_outside_period() {
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
            .generate_tra(1, fiskeridir_vessel_id, end + Duration::seconds(1))
            .await;

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
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
async fn test_trip_connects_to_tra_event_based_on_message_timestamp_if_reloading_timestamp_is_none()
{
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(fiskeridir_vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(10000, 0).unwrap();
        let end = Utc.timestamp_opt(100000, 0).unwrap();

        let mut tra =
            fiskeridir_rs::ErsTra::test_default(1, Some(fiskeridir_vessel_id.0 as u64), start);
        tra.message_info
            .set_message_timestamp(start + Duration::seconds(10));
        tra.reloading_timestamp = None;
        tra.reloading_date = None;
        tra.reloading_time = None;
        helper.db.db.add_ers_tra(vec![tra]).await.unwrap();

        let ers_trip = helper
            .generate_ers_trip(fiskeridir_vessel_id, &start, &end)
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);

        assert_eq!(trips[0].events.len(), 3);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsPor);
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

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
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

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
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

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::Landing);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::Haul);
        assert_eq!(trips[0].events[5].event_type, VesselEventType::ErsPor);
    })
    .await;
}

#[tokio::test]
async fn test_landings_trip_only_contains_landing_events() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(1000000000, 0).unwrap();

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

        helper.generate_landings_trip(vessel_id, &start, &end).await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0].events.len(), 1);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::Landing);
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
            .get_trips(TripsParameters::default(), Some(token))
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
            .get_trips(TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 2);

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
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
            .get_trips(TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 2);

        let token = helper.bw_helper.get_bw_token_with_policies(vec![]);
        let response = helper
            .app
            .get_trips(TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_offset() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;
        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    offset: Some(1),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_limit() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;
        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    limit: Some(1),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_orders_ascendingly() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;
        let trip2 = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    ordering: Some(Ordering::Asc),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], trip);
        assert_eq!(trips[1], trip2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_orders_descendingly() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;
        let trip2 = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    ordering: Some(Ordering::Desc),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], trip2);
        assert_eq!(trips[1], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_delivery_point() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let start = Utc.timestamp_opt(1, 0).unwrap();

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::seconds(1);
        landing.delivery_point = DeliveryPoint {
            id: Some(DeliveryPointId::try_from("FKAI").unwrap()),
            org_id: None,
            nationality_code: None,
        };
        helper.db.add_landings(vec![landing]).await;
        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start,
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    delivery_points: Some(vec!["FKAI".into()]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_start_date() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(20, 0).unwrap();

        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start,
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    start_date: Some(start),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_end_date() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let end = Utc.timestamp_opt(25, 0).unwrap();

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    end_date: Some(end),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_returns_bad_request_if_start_date_is_after_end_date() {
    test(|helper| async move {
        let start = Utc.timestamp_opt(30, 0).unwrap();
        let end = Utc.timestamp_opt(25, 0).unwrap();

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    start_date: Some(start),
                    end_date: Some(end),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_start_date() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let trip2 = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    sorting: Some(TripSorting::StartDate),
                    ordering: Some(Ordering::Asc),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], trip);
        assert_eq!(trips[1], trip2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_end_date() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let trip2 = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    sorting: Some(TripSorting::StopDate),
                    ordering: Some(Ordering::Asc),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], trip);
        assert_eq!(trips[1], trip2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_weight() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let start = Utc.timestamp_opt(1, 0).unwrap();
        let start2 = Utc.timestamp_opt(20, 0).unwrap();

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::seconds(1);
        landing.product.living_weight = Some(10.0);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id.0));
        landing2.landing_timestamp = start2 + Duration::seconds(1);
        landing2.product.living_weight = Some(20.0);
        helper.db.add_landings(vec![landing, landing2]).await;

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start,
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let trip2 = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start2,
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    sorting: Some(TripSorting::Weight),
                    ordering: Some(Ordering::Asc),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);

        // The trip is associatd with all landings on creation
        assert_eq!(trips[0].trip_id, trip.trip_id);
        assert_eq!(trips[1], trip2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_gear_group_ids() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let start = Utc.timestamp_opt(1, 0).unwrap();
        let start2 = Utc.timestamp_opt(20, 0).unwrap();

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::seconds(1);
        landing.gear.group = GearGroup::Not;
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id.0));
        landing2.landing_timestamp = start2 + Duration::seconds(1);
        landing2.gear.group = GearGroup::Garn;

        helper.db.add_landings(vec![landing, landing2]).await;
        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start2,
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start,
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    gear_group_ids: Some(vec![GearGroupId(GearGroup::Not)]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_species_group_ids() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);

        let start = Utc.timestamp_opt(1, 0).unwrap();
        let start2 = Utc.timestamp_opt(20, 0).unwrap();

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.landing_timestamp = start + Duration::seconds(1);
        landing.product.species.group_code = SpeciesGroup::Uer;
        landing.product.living_weight = Some(10.0);
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id.0));
        landing2.landing_timestamp = start2 + Duration::seconds(1);
        landing2.product.species.group_code = SpeciesGroup::Sei;
        landing2.product.living_weight = Some(20.0);

        helper.db.add_landings(vec![landing, landing2]).await;

        helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start2,
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start,
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    species_group_ids: Some(vec![SpeciesGroupId(SpeciesGroup::Uer)]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_vessel_length_groups() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(2);

        let mut landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.vessel.length = Some(9.00);

        let mut landing2 = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id2.0));
        landing2.vessel.length = Some(13.00);

        helper.db.add_landings(vec![landing, landing2]).await;

        let start = Utc.timestamp_opt(1, 0).unwrap();
        let start2 = Utc.timestamp_opt(20, 0).unwrap();

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &start,
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        helper
            .generate_ers_trip(
                fiskeridir_vessel_id2,
                &start2,
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    vessel_length_groups: Some(vec![utils::VesselLengthGroup(
                        VesselLengthGroup::UnderEleven,
                    )]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_fiskeridir_vessel_ids() {
    test(|mut helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(1);
        let fiskeridir_vessel_id2 = FiskeridirVesselId(2);

        let trip = helper
            .generate_ers_trip(
                fiskeridir_vessel_id,
                &Utc.timestamp_opt(1, 0).unwrap(),
                &Utc.timestamp_opt(10, 0).unwrap(),
            )
            .await;

        helper
            .generate_ers_trip(
                fiskeridir_vessel_id2,
                &Utc.timestamp_opt(20, 0).unwrap(),
                &Utc.timestamp_opt(30, 0).unwrap(),
            )
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    fiskeridir_vessel_ids: Some(vec![fiskeridir_vessel_id]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_hauls() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(1000000000, 0).unwrap();

        let haul = helper
            .db
            .generate_haul(
                vessel_id,
                &(start + Duration::hours(1)),
                &(end - Duration::hours(1)),
            )
            .await;

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        let hauls = &trips[0].hauls;
        assert_eq!(hauls.len(), 1);
        assert_eq!(hauls[0], haul);
    })
    .await;
}
