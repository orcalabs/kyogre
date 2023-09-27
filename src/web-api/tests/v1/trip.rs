use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::{DeliveryPointId, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{levels::*, Ordering, TripSorting, TripSpecification, VesselEventType};
use web_api::routes::utils::{self, GearGroupId, SpeciesGroupId};
use web_api::routes::v1::trip::{Trip, TripsOfVesselParameters, TripsParameters};

#[tokio::test]
async fn test_trip_of_landing_returns_none_of_no_trip_is_connected_to_given_landing_id() {
    test(|helper, _builder| async move {
        let response = helper
            .app
            .get_trip_of_landing(&"1-7-0-0".try_into().unwrap())
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_does_not_return_trip_outside_landing_timestamp() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).landings(1).trips(1).build().await;

        let response = helper
            .app
            .get_trip_of_landing(&state.landings[0].landing_id)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Option<Trip> = response.json().await.unwrap();
        assert!(body.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_does_not_return_trip_of_other_vessels() {
    test(|helper, builder| async move {
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

        let response = helper
            .app
            .get_trip_of_landing(&state.landings[0].landing_id)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: Trip = response.json().await.unwrap();
        assert_eq!(state.trips[0], trip);
    })
    .await;
}

#[tokio::test]
async fn test_trip_of_landing_returns_all_hauls_and_landings_connected_to_trip() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .hauls(1)
            .build()
            .await;

        let response = helper
            .app
            .get_trip_of_landing(&state.landings[0].landing_id)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: Trip = response.json().await.unwrap();
        assert_eq!(state.trips[0], body);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_only_returns_trips_of_specified_vessel() {
    test(|helper, builder| async move {
        let state = builder.vessels(2).trips(2).build().await;

        let response = helper
            .app
            .get_trips_of_vessel(
                state.vessels[0].fiskeridir.id,
                TripsOfVesselParameters::default(),
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_filters_by_limit() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        let params = TripsOfVesselParameters {
            limit: Some(1),
            offset: None,
            ordering: Some(Ordering::Asc),
        };

        let response = helper
            .app
            .get_trips_of_vessel(state.vessels[0].fiskeridir.id, params, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_filters_by_offset() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        let params = TripsOfVesselParameters {
            limit: None,
            offset: Some(1),
            ordering: Some(Ordering::Desc),
        };

        let response = helper
            .app
            .get_trips_of_vessel(state.vessels[0].fiskeridir.id, params, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_of_vessel_orders_by_period() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;
        let params = TripsOfVesselParameters {
            limit: None,
            offset: None,
            ordering: Some(Ordering::Asc),
        };

        let response = helper
            .app
            .get_trips_of_vessel(state.vessels[0].fiskeridir.id, params, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_first_ers_data_triggers_trip_assembler_switch_to_ers() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).landing_trips(1).trips(1).build().await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_all_events_within_trip_period_ordered_ascendingly() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .landings(1)
            .tra(1)
            .hauls(1)
            .build()
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
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_events_are_isolated_per_vessel() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(2)
            .trips(2)
            .landings(2)
            .tra(2)
            .hauls(2)
            .build()
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    fiskeridir_vessel_ids: Some(vec![state.vessels[0].fiskeridir.id]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0], state.trips[0]);

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    fiskeridir_vessel_ids: Some(vec![state.vessels[1].fiskeridir.id]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 6);
        assert_eq!(trips[0], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_include_events_outside_period() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .landings(1)
            .tra(1)
            .hauls(1)
            .trips(1)
            .build()
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

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
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .tra(1)
            .modify(|t| {
                t.tra.reloading_timestamp = None;
                t.tra.reloading_date = None;
                t.tra.reloading_time = None;
            })
            .build()
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

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
    test(|helper, builder| async move {
        let start_port = "NOTOS".to_string();
        let end_port = "DENOR".to_string();
        let state = builder
            .vessels(1)
            .trips(1)
            .modify(|t| match &mut t.trip_specification {
                TripSpecification::Ers { dep, por } => {
                    dep.port.code = Some(start_port.clone());
                    por.port.code = Some(end_port.clone());
                }
                TripSpecification::Landing {
                    start_landing: _,
                    end_landing: _,
                } => unreachable!(),
            })
            .build()
            .await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

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
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(3).adjacent().build().await;

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
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .landing_trips(1)
            .tra(1)
            .hauls(1)
            .build()
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
        assert_eq!(trips, state.trips);
        assert_eq!(trips.len(), 2);
        assert_eq!(trips[1].events.len(), 1);
        assert_eq!(trips[1].events[0].event_type, VesselEventType::Landing);
    })
    .await;
}

#[tokio::test]
async fn test_trip_contains_fishing_facilities() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .build()
            .await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_trips(TripsParameters::default(), Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips, state.trips);
        assert_eq!(trips[0].fishing_facilities.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_trip_does_not_return_fishing_facilities_without_token() {
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .build()
            .await;

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
    test(|helper, builder| async move {
        builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(1)
            .build()
            .await;

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
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

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
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_limit() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;
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
        assert_eq!(trips[0], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_orders_ascendingly() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;
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
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_orders_descendingly() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

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
        assert_eq!(trips[0], state.trips[1]);
        assert_eq!(trips[1], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_delivery_point() {
    test(|helper, builder| async move {
        let delivery_point = DeliveryPointId::try_from("FKAI").unwrap();

        let state = builder
            .vessels(1)
            .trips(2)
            .landings(1)
            .modify(|v| {
                v.landing.delivery_point.id = Some(delivery_point.clone());
            })
            .build()
            .await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    delivery_points: Some(vec![delivery_point.into_inner()]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_start_date() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    start_date: Some(state.trips[0].period.start() + Duration::seconds(1)),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_end_date() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;
        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    end_date: Some(state.trips[0].period.end() + Duration::seconds(1)),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
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
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;
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
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_end_date() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(2).build().await;
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
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_sorts_by_weight() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(2)
            .landings(2)
            .modify_idx(|i, v| {
                v.landing.product.living_weight = Some(i as f64 + 1.0);
            })
            .build()
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
        assert_eq!(trips[0], state.trips[0]);
        assert_eq!(trips[1], state.trips[1]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_gear_group_ids() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(2)
            .landings(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.landing.gear.group = GearGroup::Not;
                } else {
                    v.landing.gear.group = GearGroup::Garn;
                }
            })
            .build()
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
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_species_group_ids() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(2)
            .landings(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.landing.product.species.group_code = SpeciesGroup::Uer;
                } else {
                    v.landing.product.species.group_code = SpeciesGroup::Sei;
                }
            })
            .build()
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
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_vessel_length_groups() {
    test(|helper, builder| async move {
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
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_filter_by_fiskeridir_vessel_ids() {
    test(|helper, builder| async move {
        let state = builder.vessels(2).trips(2).build().await;

        let response = helper
            .app
            .get_trips(
                TripsParameters {
                    fiskeridir_vessel_ids: Some(vec![state.vessels[0].fiskeridir.id]),
                    ..Default::default()
                },
                None,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0], state.trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_trips_contains_hauls() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).hauls(1).build().await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
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
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).landings(3).build().await;

        let response = helper.app.get_trips(TripsParameters::default(), None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trips: Vec<Trip> = response.json().await.unwrap();
        let landing_ids = &trips[0].landing_ids;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips, state.trips);
        assert_eq!(landing_ids.len(), 3);
        assert_eq!(
            *landing_ids,
            state
                .landings
                .into_iter()
                .map(|v| v.landing_id)
                .collect::<Vec<LandingId>>()
        );
    })
    .await;
}
