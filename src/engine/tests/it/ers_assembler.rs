use crate::helper::*;
use chrono::{TimeZone, Utc};
use engine::*;
use kyogre_core::*;

#[tokio::test]
async fn test_logs_conflicts() {
    test(|helper, builder| async move {
        let departure = Utc.timestamp_opt(10, 0).unwrap();
        let arrival = Utc.timestamp_opt(20, 0).unwrap();
        let departure2 = Utc.timestamp_opt(30, 0).unwrap();
        let arrival2 = Utc.timestamp_opt(40, 0).unwrap();

        builder
            .vessels(1)
            .dep(1)
            .modify(|v| v.dep.set_departure_timestamp(departure2))
            .por(1)
            .modify(|v| v.por.set_arrival_timestamp(arrival2))
            .new_cycle()
            .dep(1)
            .modify(|v| v.dep.set_departure_timestamp(departure))
            .por(1)
            .modify(|v| v.por.set_arrival_timestamp(arrival))
            .build()
            .await;

        let mut log = helper.adapter().trip_assembler_log().await;
        assert_eq!(log.len(), 2);
        let log = log.pop().unwrap();
        assert!(log.is_conflict());
        assert_eq!(log.conflict.unwrap().timestamp(), departure.timestamp());
        assert_eq!(
            log.conflict_vessel_event_timestamp.unwrap().timestamp(),
            departure.timestamp()
        );
        assert_eq!(
            log.conflict_vessel_event_type_id.unwrap(),
            VesselEventType::ErsDep,
        );
        assert_eq!(log.conflict_vessel_event_id.unwrap(), 3);
    })
    .await
}
#[tokio::test]
async fn test_logs_actions() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .dep(1)
            .por(1)
            .build()
            .await;

        let mut log = helper.adapter().trip_assembler_log().await;
        assert_eq!(log.len(), 2);
        let log2 = log.pop().unwrap();
        let log1 = log.pop().unwrap();

        assert!(log1.calculation_timer_prior.is_none());
        assert!(!log1.is_conflict());
        assert!(log1.prior_trip_vessel_events.is_empty());
        assert_eq!(log1.new_vessel_events.len(), 2);

        assert_eq!(log1.new_vessel_events[0].vessel_event_id, 1);
        assert_eq!(
            log1.new_vessel_events[0].event_type,
            VesselEventType::ErsDep
        );
        assert_eq!(
            log1.new_vessel_events[0].timestamp.timestamp(),
            state.dep[0].timestamp.timestamp()
        );
        assert_eq!(log1.new_vessel_events[1].vessel_event_id, 2);
        assert_eq!(
            log1.new_vessel_events[1].event_type,
            VesselEventType::ErsPor
        );
        assert_eq!(
            log1.new_vessel_events[1].timestamp.timestamp(),
            state.por[0].timestamp.timestamp()
        );

        assert!(log2.calculation_timer_prior.is_some());
        assert!(!log2.is_conflict());
        assert_eq!(log2.prior_trip_vessel_events.len(), 2);

        assert_eq!(log2.prior_trip_vessel_events[0].vessel_event_id, 1);
        assert_eq!(
            log2.prior_trip_vessel_events[0].event_type,
            VesselEventType::ErsDep
        );
        assert_eq!(
            log2.prior_trip_vessel_events[0].timestamp.timestamp(),
            state.dep[0].timestamp.timestamp()
        );

        assert_eq!(log2.prior_trip_vessel_events[1].vessel_event_id, 2);
        assert_eq!(
            log2.prior_trip_vessel_events[1].event_type,
            VesselEventType::ErsPor
        );
        assert_eq!(
            log2.prior_trip_vessel_events[1].timestamp.timestamp(),
            state.por[0].timestamp.timestamp()
        );

        assert_eq!(log2.new_vessel_events.len(), 2);
        assert_eq!(log2.new_vessel_events[0].vessel_event_id, 3);
        assert_eq!(
            log2.new_vessel_events[0].event_type,
            VesselEventType::ErsDep
        );
        assert_eq!(
            log2.new_vessel_events[0].timestamp.timestamp(),
            state.dep[1].timestamp.timestamp()
        );

        assert_eq!(log2.new_vessel_events[1].vessel_event_id, 4);
        assert_eq!(
            log2.new_vessel_events[1].event_type,
            VesselEventType::ErsPor
        );
        assert_eq!(
            log2.new_vessel_events[1].timestamp.timestamp(),
            state.por[1].timestamp.timestamp()
        );
    })
    .await;
}

#[tokio::test]
async fn test_produces_new_trips_without_replacing_existing_ones() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .dep(1)
            .por(1)
            .build()
            .await;

        let trip = &state.trips[0];
        let trip2 = &state.trips[1];
        assert_eq!(state.trips.len(), 2);

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);
        assert_eq!(trip.landing_coverage.start(), state.dep[0].timestamp);
        assert_eq!(trip.landing_coverage.end(), state.dep[1].timestamp);

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.dep[1].timestamp);
        assert_eq!(
            trip2.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[1].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_produces_no_trips_with_no_new_departures_or_arrivals() {
    test(|helper, builder| async move {
        builder.vessels(1).dep(1).por(1).build().await;
        let state = helper.builder().await.build().await;
        assert_eq!(state.trips.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_extends_most_recent_trip_with_new_arrival() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .por(1)
            .build()
            .await;

        let trip = &state.trips[0];
        assert_eq!(state.trips.len(), 1);

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[1].timestamp);
        assert_eq!(trip.landing_coverage.start(), state.dep[0].timestamp);
        assert_eq!(
            trip.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[1].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_handles_conflict_correctly() {
    test(|helper, builder| async move {
        let departure = Utc.timestamp_opt(10, 0).unwrap();
        let arrival = Utc.timestamp_opt(20, 0).unwrap();
        let departure2 = Utc.timestamp_opt(30, 0).unwrap();
        let arrival2 = Utc.timestamp_opt(40, 0).unwrap();
        let departure3 = Utc.timestamp_opt(22, 0).unwrap();
        let arrival3 = Utc.timestamp_opt(27, 0).unwrap();

        let state = builder
            .vessels(1)
            .dep(2)
            .modify_idx(|i, v| {
                let time = if i == 0 { departure } else { departure2 };
                v.dep.set_departure_timestamp(time);
                v.dep.message_info.set_message_timestamp(time);
            })
            .por(2)
            .modify_idx(|i, v| {
                let time = if i == 0 { arrival } else { arrival2 };
                v.por.set_arrival_timestamp(time);
                v.por.message_info.set_message_timestamp(time);
            })
            .new_cycle()
            .dep(1)
            .modify(|v| {
                v.dep.set_departure_timestamp(departure3);
                v.dep.message_info.set_message_timestamp(departure3);
            })
            .por(1)
            .modify(|v| {
                v.por.set_arrival_timestamp(arrival3);
                v.por.message_info.set_message_timestamp(arrival3);
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 3);
        let trip = &state.trips[0];
        let trip2 = &state.trips[1];
        let trip3 = &state.trips[2];

        assert_eq!(trip.period.start(), departure);
        assert_eq!(trip.period.end(), arrival);
        assert_eq!(trip.landing_coverage.start(), departure);
        assert_eq!(trip.landing_coverage.end(), departure3);

        assert_eq!(trip2.period.start(), departure3);
        assert_eq!(trip2.period.end(), arrival3);
        assert_eq!(trip2.landing_coverage.start(), departure3);
        assert_eq!(trip2.landing_coverage.end(), departure2);

        assert_eq!(trip3.period.start(), departure2);
        assert_eq!(trip3.period.end(), arrival2);
        assert_eq!(trip3.landing_coverage.start(), departure2);
        assert_eq!(
            trip3.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&arrival2)
        );
        assert!(helper
            .adapter()
            .trip_calculation_timer(state.vessels[0].fiskeridir.id, TripAssemblerId::Ers)
            .await
            .unwrap()
            .unwrap()
            .conflict
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn test_is_not_affected_of_other_vessels_trips() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .vessels(1)
            .dep(1)
            .por(1)
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        let trip = &state.trips[0];
        let trip2 = &state.trips[1];

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);
        assert_eq!(trip.landing_coverage.start(), state.dep[0].timestamp);
        assert_eq!(
            trip.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[0].timestamp)
        );

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.dep[1].timestamp);
        assert_eq!(
            trip2.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[1].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_ignores_arrival_if_its_the_first_ever_event_for_a_vessel() {
    test(|_helper, builder| async move {
        let state = builder.vessels(1).por(1).dep(1).por(1).build().await;

        assert_eq!(state.trips.len(), 1);
        let trip = &state.trips[0];
        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[1].timestamp);
        assert_eq!(trip.landing_coverage.start(), state.dep[0].timestamp);
        assert_eq!(
            trip.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[1].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_does_not_panic_with_single_arrival() {
    test(|_helper, builder| async move {
        let state = builder.vessels(1).por(1).build().await;
        assert_eq!(state.trips.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_handles_dep_and_por_with_identical_timestamps() {
    test(|_helper, builder| async move {
        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let state = builder
            .vessels(1)
            .dep(2)
            .modify(|v| v.dep.set_departure_timestamp(start))
            .por(1)
            .build()
            .await;

        assert_eq!(state.trips.len(), 1);
        let trip = &state.trips[0];

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);
        assert_eq!(trip.landing_coverage.start(), state.dep[0].timestamp);
        assert_eq!(
            trip.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[0].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_other_event_types_does_not_cause_conflicts() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .hauls(1)
            .landings(1)
            .tra(1)
            .por(1)
            .build()
            .await;

        assert!(helper
            .adapter()
            .trip_calculation_timer(state.vessels[0].fiskeridir.id, TripAssemblerId::Ers)
            .await
            .unwrap()
            .unwrap()
            .conflict
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn test_ignores_arrivals_and_departures_prior_to_epoch() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .modify(|v| {
                v.dep
                    .set_departure_timestamp(Utc.timestamp_opt(-20, 0).unwrap())
            })
            .por(1)
            .modify(|v| {
                v.por
                    .set_arrival_timestamp(Utc.timestamp_opt(-10, 0).unwrap())
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_queuing_a_reset_re_creates_trips() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .base()
            .queue_trip_reset()
            .build()
            .await;

        assert_eq!(state.trips[0].trip_id.into_inner(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_reset_is_cleared_on_next_run() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .base()
            .queue_trip_reset()
            .new_cycle()
            .build()
            .await;

        assert_eq!(state.trips.len(), 1);
        assert_eq!(state.trips[0].trip_id.into_inner(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_reset_deletes_all_trips_including_non_overlaps() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .dep(1)
            .por(1)
            .base()
            .queue_trip_reset()
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        assert_eq!(state.trips[0].trip_id.into_inner(), 2);
        assert_eq!(state.trips[1].trip_id.into_inner(), 3);
    })
    .await;
}
