use crate::helper::*;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use kyogre_core::*;

#[tokio::test]
async fn test_does_not_logs_actions_on_success() {
    test(|helper, builder| async move {
        builder.vessels(1).dep(1).por(1).build().await;
        let logs = helper.adapter().trip_assembler_log().await;
        assert!(logs.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_handles_dep_and_por_on_same_timestamp2() {
    test(|_helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 2, 1, 0, 0, 0).unwrap();
        let start2 = Utc.with_ymd_and_hms(2020, 6, 1, 0, 0, 0).unwrap();
        let start3 = Utc.with_ymd_and_hms(2020, 7, 1, 0, 0, 0).unwrap();

        let state = builder
            .vessels(1)
            .dep(1)
            .modify(|d| {
                d.dep.set_departure_timestamp(start);
                d.dep.message_info.message_number = 1;
            })
            .por(2)
            .modify_idx(|i, p| {
                p.por.set_arrival_timestamp(start2);
                if i == 0 {
                    p.por.message_info.message_number = 2;
                } else {
                    p.por.message_info.message_number = 4;
                }
            })
            .dep(1)
            .modify(|d| {
                d.dep.set_departure_timestamp(start2);
                d.dep.message_info.message_number = 3;
            })
            .dep(1)
            .modify(|d| {
                d.dep.set_departure_timestamp(start3);
                d.dep.message_info.message_number = 5;
            })
            .por(1)
            .modify(|p| {
                p.por.set_arrival_timestamp(start3);
                p.por.message_info.message_number = 6;
            })
            .build()
            .await;

        let trip = &state.trips[0];
        let trip2 = &state.trips[1];
        let trip3 = &state.trips[2];
        assert_eq!(state.trips.len(), 3);

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);

        assert_eq!(
            trip.landing_coverage.start(),
            state.por[0].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
        assert_eq!(trip.landing_coverage.end(), state.por[1].timestamp);

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.end(), state.por[2].timestamp);

        assert_eq!(trip3.period.start(), state.dep[2].timestamp);
        assert_eq!(trip3.period.end(), state.por[2].timestamp);
        assert_eq!(trip3.landing_coverage.start(), state.por[2].timestamp);
        assert_eq!(
            trip3.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[2].timestamp)
        );
    })
    .await;
}
#[tokio::test]
async fn test_handles_dep_and_por_on_same_timestamp() {
    test(|_helper, builder| async move {
        let start = Utc.with_ymd_and_hms(2020, 2, 1, 0, 0, 0).unwrap();
        let start2 = Utc.with_ymd_and_hms(2020, 6, 1, 0, 0, 0).unwrap();

        let state = builder
            .vessels(1)
            .dep(1)
            .modify(|d| {
                d.dep.set_departure_timestamp(start);
                d.dep.message_info.message_number = 1;
            })
            .por(1)
            .modify(|p| {
                p.por
                    .set_arrival_timestamp(start + ERS_LANDING_COVERAGE_OFFSET);
                p.por.message_info.message_number = 2;
            })
            .dep(1)
            .modify(|d| {
                d.dep.set_departure_timestamp(start2);
                d.dep.message_info.message_number = 4;
            })
            .por(2)
            .modify_idx(|i, p| {
                p.por.set_arrival_timestamp(start2);
                if i == 0 {
                    p.por.message_info.message_number = 3;
                } else {
                    p.por.message_info.message_number = 5;
                }
            })
            .build()
            .await;

        let trip = &state.trips[0];
        let trip2 = &state.trips[1];
        assert_eq!(state.trips.len(), 2);

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[1].timestamp);
        assert_eq!(
            trip.landing_coverage.start(),
            state.por[1].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
        assert_eq!(trip.landing_coverage.end(), state.por[2].timestamp);

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[2].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.por[2].timestamp);
        assert_eq!(
            trip2.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[2].timestamp)
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
        assert_eq!(trip.landing_coverage.start(), state.por[0].timestamp);
        assert_eq!(trip.landing_coverage.end(), state.por[1].timestamp);

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.por[1].timestamp);
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
async fn test_does_not_recompute_prior_trip_with_new_departure_event() {
    test(|_helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .dep(1)
            .build()
            .await;

        let trip = &state.trips[0];
        assert_eq!(state.trips.len(), 1);
        assert_eq!(trip.trip_id.into_inner(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_extends_most_recent_trip_with_new_arrival() {
    test(|_helper, builder| async move {
        let state = builder
            .data_increment(ERS_LANDING_COVERAGE_OFFSET)
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
        assert_eq!(
            trip.landing_coverage.start(),
            state.por[1].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
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
        assert_eq!(trip.landing_coverage.start(), arrival);
        assert_eq!(trip.landing_coverage.end(), arrival3);

        assert_eq!(trip2.period.start(), departure3);
        assert_eq!(trip2.period.end(), arrival3);
        assert_eq!(trip2.landing_coverage.start(), arrival3);
        assert_eq!(trip2.landing_coverage.end(), arrival2);

        assert_eq!(trip3.period.start(), departure2);
        assert_eq!(trip3.period.end(), arrival2);
        assert_eq!(trip3.landing_coverage.start(), arrival2);
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
        assert_eq!(trip.landing_coverage.start(), state.por[0].timestamp);
        assert_eq!(
            trip.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[0].timestamp)
        );

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.por[1].timestamp);
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
        assert_eq!(trip.landing_coverage.start(), state.por[1].timestamp);
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
        assert_eq!(
            trip.landing_coverage.start(),
            state.por[0].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
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
            .tra(1)
            .por(1)
            .landings(1)
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

#[tokio::test]
async fn test_handles_prior_trip_shorter_than_landing_coverage_offset() {
    test(|_helper, builder| async move {
        let state = builder
            .data_increment(ERS_LANDING_COVERAGE_OFFSET)
            .trip_duration(ERS_LANDING_COVERAGE_OFFSET - Duration::seconds(1))
            .vessels(1)
            .trips(1)
            .dep(1)
            .por(1)
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        let trip = &state.trips[0];
        let trip2 = &state.trips[1];

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);
        assert_eq!(trip.landing_coverage.start(), state.por[0].timestamp);
        assert_eq!(
            trip.landing_coverage.end(),
            state.por[1].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(
            trip2.landing_coverage.start(),
            state.por[1].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
        assert_eq!(
            trip2.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[1].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_handles_trip_shorter_than_landing_coverage_offset() {
    test(|_helper, builder| async move {
        let state = builder
            .data_increment(ERS_LANDING_COVERAGE_OFFSET - Duration::hours(1))
            .trip_duration(ERS_LANDING_COVERAGE_OFFSET)
            .vessels(1)
            .trips(1)
            .dep(1)
            .por(1)
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        let trip = &state.trips[0];
        let trip2 = &state.trips[1];

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);
        assert_eq!(
            trip.landing_coverage.start(),
            state.por[0].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
        assert_eq!(trip.landing_coverage.end(), state.por[1].timestamp);

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[1].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.por[1].timestamp);
        assert_eq!(
            trip2.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[1].timestamp)
        );
    })
    .await;
}

#[tokio::test]
async fn test_handles_extending_trip_shorter_than_landing_coverage_offset() {
    test(|_helper, builder| async move {
        let state = builder
            .data_increment(ERS_LANDING_COVERAGE_OFFSET - Duration::hours(5))
            .trip_duration(ERS_LANDING_COVERAGE_OFFSET)
            .vessels(1)
            .trips(1)
            .dep(1)
            .por(1)
            .new_cycle()
            .por(1)
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        let trip = &state.trips[0];
        let trip2 = &state.trips[1];

        assert_eq!(trip.period.start(), state.dep[0].timestamp);
        assert_eq!(trip.period.end(), state.por[0].timestamp);
        assert_eq!(
            trip.landing_coverage.start(),
            state.por[0].timestamp - ERS_LANDING_COVERAGE_OFFSET
        );
        assert_eq!(trip.landing_coverage.end(), state.por[2].timestamp);

        assert_eq!(trip2.period.start(), state.dep[1].timestamp);
        assert_eq!(trip2.period.end(), state.por[2].timestamp);
        assert_eq!(trip2.landing_coverage.start(), state.por[2].timestamp);
        assert_eq!(
            trip2.landing_coverage.end(),
            ers_last_trip_landing_coverage_end(&state.por[2].timestamp)
        );
    })
    .await;
}
