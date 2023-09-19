use crate::helper::test;
use chrono::{DateTime, Duration, TimeZone, Utc};
use kyogre_core::*;

#[tokio::test]
async fn test_dock_point_precision_extends_start_and_end_of_trip() {
    test(|_helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(1000000000, 0).unwrap();
        let mut state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
                v.trip_specification.set_ports("NOTOS");
            })
            .precision(PrecisionId::DockPoint)
            .build()
            .await;

        let period_precision = state.trips[0].period_precision.take().unwrap();
        assert!(period_precision.start() < start);
        assert!(period_precision.end() > end);
    })
    .await
}

#[tokio::test]
async fn test_port_precision_extends_start_and_end_of_trip() {
    test(|_helper, builder| async move {
        let start = Utc.timestamp_opt(10000000, 0).unwrap();
        let end = Utc.timestamp_opt(1000000000, 0).unwrap();
        let mut state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
                v.trip_specification.set_ports("NOTOS");
            })
            .precision(PrecisionId::Port)
            .build()
            .await;

        let period_precision = state.trips[0].period_precision.take().unwrap();
        assert!(period_precision.start() < start);
        assert!(period_precision.end() > end);
    })
    .await
}

#[tokio::test]
async fn tests_trips_runs_precision_on_existing_unprocessed_trips() {
    test(|_helper, builder| async move {
        let start = Utc.timestamp_opt(1000000, 0).unwrap();
        let end = Utc.timestamp_opt(2000000, 0).unwrap();
        let state = builder
            .vessels(1)
            .clear_trip_precision()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .new_cycle()
            .ais_positions(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.position.msgtime = start - Duration::seconds(1);
                    v.position.latitude = 54.5;
                    v.position.longitude = 8.883333333333333;
                }
                1 => {
                    v.position.msgtime = end - Duration::seconds(1);
                    v.position.latitude = 54.5;
                    v.position.longitude = 8.883333333333333;
                }
                2 => {
                    v.position.msgtime = end + Duration::seconds(1);
                    v.position.latitude = 54.5;
                    v.position.longitude = 8.883333333333333;
                }
                _ => unreachable!(),
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 1);
        assert!(state.trips[0].period_precision.is_some());
    })
    .await;
}

#[tokio::test]
async fn test_extending_trip_overlapping_last_trips_landing_coverage_succeeds() {
    test(|_helper, builder| async move {
        let start = DateTime::parse_from_rfc3339("2023-07-20 23:29:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2023-08-16 09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let start2 = DateTime::parse_from_rfc3339("2023-08-18T18:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end2 = DateTime::parse_from_rfc3339("2023-09-20T08:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let mut state = builder
            .vessels(1)
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .new_cycle()
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start2);
                v.trip_specification.set_end(end2);
            })
            .precision(PrecisionId::Port)
            .build()
            .await;

        let period_precision = state.trips[1].period_precision.take().unwrap();
        assert!(period_precision.start() < start2);
        assert!(period_precision.end() > end2);
    })
    .await;
}
