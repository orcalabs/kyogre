use super::helper::{test, test_with_cache};
use chrono::{DateTime, Duration, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::{DeliveryPointId, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use http_client::StatusCode;
use kyogre_core::{FiskeridirVesselId, Ordering, TripSorting, VesselEventType};
use uuid::Uuid;
use web_api::{error::ErrorDiscriminants, routes::v1::trip::TripsParameters};

#[tokio::test]
async fn test_trips_contains_hauls_added_after_trip_creation() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .new_cycle()
            .hauls(1)
            .build()
            .await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].hauls.len(), 1);
        assert_eq!(trips[0].hauls, state.hauls);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_refreshed_fishing_facilities() {
    test(|mut helper, builder| async move {
        let tool_id = Uuid::new_v4();
        let state = builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .modify(|v| {
                v.facility.tool_id = tool_id;
                v.facility.imo = Some(1);
            })
            .new_cycle()
            .fishing_facilities(1)
            .modify(|v| {
                v.facility.tool_id = tool_id;
                v.facility.imo = Some(2);
            })
            .build()
            .await;

        helper.app.login_user();

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 1);
        assert_eq!(trips[0].fishing_facilities, state.fishing_facilities);
        assert_eq!(trips[0].fishing_facilities[0].imo.unwrap(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_refreshed_hauls() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .hauls(1)
            .modify(|v| {
                v.dca.message_info.message_id = 1;
                v.dca.message_version = 1;
                v.dca.catch.species.living_weight = Some(10);
            })
            .new_cycle()
            .hauls(1)
            .modify(|v| {
                v.dca.message_info.message_id = 1;
                v.dca.message_version = 2;
                v.dca.catch.species.living_weight = Some(20);
            })
            .build()
            .await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].hauls.len(), 1);
        assert_eq!(trips[0].hauls, state.hauls);
        assert_eq!(trips[0].hauls[0].catches[0].living_weight, 20);
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_returns_none_of_no_trip_is_connected_to_given_landing_id() {
    test_with_cache(|helper, _builder| async move {
        helper.refresh_cache().await;

        let trip = helper
            .app
            .get_trip_of_landing(&"1-7-0-0".parse().unwrap())
            .await
            .unwrap();
        assert!(trip.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_does_not_return_trip_outside_landing_timestamp() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).landings(1).trips(1).build().await;

        helper.refresh_cache().await;

        let trip = helper
            .app
            .get_trip_of_landing(&state.landings[0].id)
            .await
            .unwrap();
        assert!(trip.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_does_not_return_trip_of_other_vessels() {
    test_with_cache(|helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(20000000, 0).unwrap();

        let state = builder
            .vessels(1)
            .landings(1)
            .modify(|l| l.landing.landing_timestamp = start + Duration::seconds(1))
            .base()
            .vessels(1)
            .trips(1)
            .modify(|t| {
                t.trip_specification.set_start(start);
                t.trip_specification.set_end(end);
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trip = helper
            .app
            .get_trip_of_landing(&state.landings[0].id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_returns_all_hauls_and_landings_connected_to_trip() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .hauls(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let trip = helper
            .app
            .get_trip_of_landing(&state.landings[0].id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_first_ers_data_triggers_trip_assembler_switch_to_ers() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).landing_trips(1).trips(1).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_all_events_within_trip_period_ordered_ascendingly() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .tra(1)
            .hauls(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::Landing);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::Haul);
        assert_eq!(trips[0].events[5].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_events_are_isolated_per_vessel() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(2)
            .trips(2)
            .landings(2)
            .tra(2)
            .hauls(2)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                fiskeridir_vessel_ids: Some(vec![state.vessels[0].fiskeridir.id]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0], state.trips[0]);

        let trips = helper
            .app
            .get_trips(TripsParameters {
                fiskeridir_vessel_ids: Some(vec![state.vessels[1].fiskeridir.id]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_include_events_outside_period() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .landings(1)
            .tra(1)
            .hauls(1)
            .trips(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 2);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trip_connects_to_tra_event_based_on_message_timestamp_if_reloading_timestamp_is_none()
{
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .tra(1)
            .modify(|t| {
                t.tra._reloading_timestamp = None;
                t.tra.reloading_date = None;
                t.tra.reloading_time = None;
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 3);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_returns_correct_ports() {
    test_with_cache(|helper, builder| async move {
        let start_port = "NOTOS".to_string();
        let end_port = "DENOR".to_string();
        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|t| match &mut t.trip_specification {
                TripSpecification::Ers { dep, por } => {
                    dep.port.code = Some(start_port.parse().unwrap());
                    por.port.code = Some(end_port.parse().unwrap());
                }
                TripSpecification::Landing {
                    start_landing: _,
                    end_landing: _,
                } => unreachable!(),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[0].start_port_id.clone().unwrap(), start_port);
        assert_eq!(trips[0].end_port_id.clone().unwrap(), end_port);
    })
    .await;
}

#[tokio::test]
async fn test_trip_contains_correct_arrival_and_departure_with_adjacent_trips_with_equal_start_and_stop(
) {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(3).adjacent().build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(trips.len(), 3);
        assert_eq!(trips, state.trips);
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
async fn test_landings_trip_only_contains_landing_events() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .landing_trips(1)
            .tra(1)
            .hauls(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(trips, state.trips);
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[1].events.len(), 1);
        assert_eq!(trips[1].events[0].event_type, VesselEventType::Landing);
    })
    .await;
}

#[tokio::test]
async fn test_trip_contains_fishing_facilities() {
    test_with_cache(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .build()
            .await;

        helper.refresh_cache().await;

        helper.app.login_user();
        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips, state.trips);
        assert_eq!(trips[0].fishing_facilities.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_trip_does_not_return_fishing_facilities_without_token() {
    test_with_cache(|helper, builder| async move {
        builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_trip_does_not_return_fishing_facilities_without_read_fishing_facility() {
    test_with_cache(|mut helper, builder| async move {
        builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .build()
            .await;

        helper.refresh_cache().await;

        helper.app.login_user_with_policies(vec![]);
        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_offset() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                offset: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_limit() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_orders_ascendingly() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_orders_descendingly() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ordering: Some(Ordering::Desc),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], state.trips[1]);
        assert_eq!(trips[1], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_delivery_point() {
    test_with_cache(|helper, builder| async move {
        let delivery_point: DeliveryPointId = "FKAI".parse().unwrap();

        let state = builder
            .vessels(1)
            .trips(2)
            .landings(1)
            .modify(|v| {
                v.landing.delivery_point.id = Some(delivery_point.clone());
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                delivery_points: Some(vec![delivery_point.into_inner()]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_start_date() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                start_date: Some(state.trips[0].period.start() + Duration::seconds(1)),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_end_date() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                end_date: Some(state.trips[0].period.end() + Duration::seconds(1)),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_returns_bad_request_if_start_date_is_after_end_date() {
    test(|helper, _builder| async move {
        let start = Utc.timestamp_opt(30, 0).unwrap();
        let end = Utc.timestamp_opt(25, 0).unwrap();

        let error = helper
            .app
            .get_trips(TripsParameters {
                start_date: Some(start),
                end_date: Some(end),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::StartAfterEnd);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_end_date() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                sorting: Some(TripSorting::StopDate),
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_weight() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(2)
            .landings(2)
            .modify_idx(|i, v| {
                v.landing.product.living_weight = Some(i as f64 + 1.0);
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                sorting: Some(TripSorting::Weight),
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_gear_group_ids() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(2)
            .landings(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.landing.gear.group = GearGroup::Seine;
                } else {
                    v.landing.gear.group = GearGroup::Net;
                }
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                gear_group_ids: Some(vec![GearGroup::Seine]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_species_group_ids() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(2)
            .landings(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.landing.product.species.group_code = SpeciesGroup::GoldenRedfish;
                } else {
                    v.landing.product.species.group_code = SpeciesGroup::Saithe;
                }
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                species_group_ids: Some(vec![SpeciesGroup::GoldenRedfish]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_vessel_length_groups() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.fiskeridir.length = 9.00;
                } else {
                    v.fiskeridir.length = 13.00;
                }
            })
            .trips(2)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                vessel_length_groups: Some(vec![VesselLengthGroup::UnderEleven]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_fiskeridir_vessel_ids() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(2).trips(2).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                fiskeridir_vessel_ids: Some(vec![state.vessels[0].fiskeridir.id]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_hauls() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).hauls(1).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();
        let hauls = &trips[0].hauls;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(hauls.len(), 1);
        assert_eq!(*hauls, state.hauls);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_landing_ids() {
    test_with_cache(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).landings(3).build().await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        let landing_ids = &trips[0].landing_ids;
        assert_eq!(trips.len(), 1);
        assert_eq!(trips, state.trips);
        assert_eq!(landing_ids.len(), 3);
        assert_eq!(
            *landing_ids,
            state
                .landings
                .into_iter()
                .map(|v| v.id)
                .collect::<Vec<LandingId>>()
        );
    })
    .await;
}

#[tokio::test]
async fn test_trips_connects_to_existing_landings_outside_period_but_inside_landing_coverage() {
    test_with_cache(|helper, builder| async move {
        let start: DateTime<Utc> = "2000-01-05T00:00:00Z".parse().unwrap();
        let end: DateTime<Utc> = "2000-01-07T00:00:00Z".parse().unwrap();

        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
            })
            .landings(1)
            .modify(|v| {
                v.landing.landing_timestamp = end + Duration::days(2);
            })
            .new_cycle()
            .base()
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
            })
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        let landing_ids = &trips[0].landing_ids;

        assert_eq!(trips.len(), 1);
        assert_eq!(landing_ids.len(), 1);
        assert_eq!(trips, state.trips);
        assert_eq!(
            *landing_ids,
            state
                .landings
                .into_iter()
                .map(|v| v.id)
                .collect::<Vec<LandingId>>()
        );
    })
    .await;
}

#[tokio::test]
async fn test_trips_returns_track_coverage() {
    test(|helper, builder| async move {
        let start: DateTime<Utc> = "2000-01-01T00:00:00Z".parse().unwrap();
        let end: DateTime<Utc> = "2000-01-01T00:10:00Z".parse().unwrap();

        builder
            .trip_data_increment(Duration::minutes(2))
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .ais_vms_positions(3)
            .modify_idx(|i, p| p.position.add_location(i as f64, i as f64))
            .build()
            .await;

        let trips = helper.app.get_trips(Default::default()).await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].track_coverage.unwrap() as i32, 30);
    })
    .await;
}

#[tokio::test]
async fn test_trips_returns_track_coverage_zero_if_no_track() {
    test(|helper, builder| async move {
        builder.vessels(1).trips(1).build().await;

        let trips = helper.app.get_trips(Default::default()).await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].track_coverage.unwrap() as i32, 0);
    })
    .await;
}
