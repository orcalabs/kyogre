use crate::helper::*;
use chrono::{Duration, TimeZone, Utc};
use kyogre_core::*;
use trip_assembler::*;

#[tokio::test]
async fn test_produces_new_trips_without_replacing_existing_ones() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = Utc.timestamp_opt(200000, 0).unwrap();
        let start2 = Utc.timestamp_opt(300000, 0).unwrap();
        let end2 = Utc.timestamp_opt(400000, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let departure2 = fiskeridir_rs::ErsDep::test_default(2, vessel_id.0 as u64, start2, 3);
        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, end2, 4);

        helper.add_ers_dep(vec![departure2.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival2.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel_id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected = vec![
            Trip {
                trip_id: TripId(1),
                period: create_date_range(&departure, &arrival),
                landing_coverage: create_date_range(&departure, &departure2),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
            Trip {
                trip_id: TripId(2),
                period: create_date_range(&departure2, &arrival2),
                landing_coverage: create_date_range(
                    &departure2,
                    &ers_last_trip_landing_coverage_end(&arrival2.arrival_timestamp),
                ),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
        ];
        assert_eq!(expected, trips);
    })
    .await;
}

#[tokio::test]
async fn test_produces_no_trips_with_no_new_departures_or_arrivals() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = Utc.timestamp_opt(200000, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        assert_eq!(trips.len(), 1);

        let expected = Trip {
            trip_id: TripId(1),
            landing_coverage: create_date_range(
                &departure,
                &ers_last_trip_landing_coverage_end(&end),
            ),
            period: create_date_range(&departure, &arrival),
            assembler_id: TripAssemblerId::Ers,
            precision_period: None,
            distance: None,
        };

        assert_eq!(expected, trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_extends_most_recent_trip_with_new_arrival() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = Utc.timestamp_opt(200000, 0).unwrap();
        let end2 = Utc.timestamp_opt(300000, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, end2, 3);

        helper.add_ers_por(vec![arrival2.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel_id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected = Trip {
            trip_id: TripId(2),
            period: create_date_range(&departure, &arrival2),
            landing_coverage: create_date_range(
                &departure,
                &ers_last_trip_landing_coverage_end(&arrival2.arrival_timestamp),
            ),
            assembler_id: TripAssemblerId::Ers,
            precision_period: None,
            distance: None,
        };
        assert_eq!(trips.len(), 1);
        assert_eq!(expected, trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_handles_conflict_correctly() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(10, 0).unwrap();
        let end = Utc.timestamp_opt(20, 0).unwrap();
        let start2 = Utc.timestamp_opt(30, 0).unwrap();
        let end2 = Utc.timestamp_opt(40, 0).unwrap();
        let start3 = Utc.timestamp_opt(22, 0).unwrap();
        let end3 = Utc.timestamp_opt(27, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        let departure2 = fiskeridir_rs::ErsDep::test_default(2, vessel_id.0 as u64, start2, 5);
        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, end2, 6);

        helper
            .add_ers_dep(vec![departure.clone(), departure2.clone()])
            .await
            .unwrap();
        helper
            .add_ers_por(vec![arrival.clone(), arrival2.clone()])
            .await
            .unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let departure3 = fiskeridir_rs::ErsDep::test_default(3, vessel_id.0 as u64, start3, 3);
        let arrival3 = fiskeridir_rs::ErsPor::test_default(3, vessel_id.0 as u64, end3, 4);

        helper.add_ers_dep(vec![departure3.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival3.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel_id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected = vec![
            Trip {
                trip_id: TripId(1),
                period: create_date_range(&departure, &arrival),
                landing_coverage: create_date_range(&departure, &departure3),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
            Trip {
                trip_id: TripId(3),
                period: create_date_range(&departure3, &arrival3),
                landing_coverage: create_date_range(&departure3, &departure2),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
            Trip {
                trip_id: TripId(4),
                period: create_date_range(&departure2, &arrival2),
                landing_coverage: create_date_range(
                    &departure2,
                    &ers_last_trip_landing_coverage_end(&arrival2.arrival_timestamp),
                ),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
        ];
        assert_eq!(trips.len(), 3);
        assert_eq!(expected, trips);
    })
    .await;
}

#[tokio::test]
async fn test_is_not_affected_of_other_vessels_trips() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let vessel_id2 = FiskeridirVesselId(12);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let departure2 = fiskeridir_rs::ErsDep::test_default(2, vessel_id2.0 as u64, start, 1);
        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id2.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure2.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival2.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel_id).await;
        let trips2 = helper.db.trips_of_vessel(vessel_id2).await;
        trips.extend(trips2);
        trips.sort_by_key(|v| v.trip_id);

        let expected = vec![
            Trip {
                trip_id: TripId(1),
                period: create_date_range(&departure, &arrival),
                landing_coverage: create_date_range(
                    &departure,
                    &ers_last_trip_landing_coverage_end(&arrival2.arrival_timestamp),
                ),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
            Trip {
                trip_id: TripId(2),
                period: create_date_range(&departure2, &arrival2),
                landing_coverage: create_date_range(
                    &departure2,
                    &ers_last_trip_landing_coverage_end(&arrival2.arrival_timestamp),
                ),
                assembler_id: TripAssemblerId::Ers,
                precision_period: None,
                distance: None,
            },
        ];
        assert_eq!(expected, trips);
    })
    .await;
}

#[tokio::test]
async fn test_ignores_arrival_if_its_the_first_ever_event_for_a_vessel() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let start2 = Utc.timestamp_opt(150000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, start, 1);
        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start2, 2);
        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, end, 3);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper
            .add_ers_por(vec![arrival, arrival2.clone()])
            .await
            .unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        let expected = Trip {
            trip_id: TripId(1),
            period: create_date_range(&departure, &arrival2),
            landing_coverage: create_date_range(
                &departure,
                &ers_last_trip_landing_coverage_end(&arrival2.arrival_timestamp),
            ),
            assembler_id: TripAssemblerId::Ers,
            precision_period: None,
            distance: None,
        };

        assert_eq!(expected, trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_does_not_panic_with_single_arrival() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 1).unwrap();

        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, start, 1);

        helper.add_ers_por(vec![arrival]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        assert_eq!(trips.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_handles_dep_and_por_with_identical_timestamps() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 3);
        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 2);
        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, start, 1);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper
            .add_ers_por(vec![arrival.clone(), arrival2])
            .await
            .unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        let expected = Trip {
            trip_id: TripId(1),
            period: create_date_range(&departure, &arrival),
            landing_coverage: create_date_range(
                &departure,
                &ers_last_trip_landing_coverage_end(&arrival.arrival_timestamp),
            ),
            assembler_id: TripAssemblerId::Ers,
            precision_period: None,
            distance: None,
        };

        assert_eq!(expected, trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_other_event_types_does_not_cause_conflicts() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 1);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        helper.db.generate_tra(1, vessel_id, start).await;
        helper.db.generate_landing(1, vessel_id, start).await;
        helper.db.generate_haul(vessel_id, &start, &end).await;

        assert_eq!(
            helper
                .adapter()
                .conflicts(TripAssemblerId::Ers)
                .await
                .unwrap()
                .len(),
            0
        );
    })
    .await;
}

#[tokio::test]
async fn test_after_resolving_conflicts_they_are_removed() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(10, 0).unwrap();
        let end = Utc.timestamp_opt(20, 0).unwrap();
        let start2 = Utc.timestamp_opt(30, 0).unwrap();
        let end2 = Utc.timestamp_opt(40, 0).unwrap();
        let start3 = Utc.timestamp_opt(22, 0).unwrap();
        let end3 = Utc.timestamp_opt(27, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        let departure2 = fiskeridir_rs::ErsDep::test_default(2, vessel_id.0 as u64, start2, 5);
        let arrival2 = fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, end2, 6);

        helper
            .add_ers_dep(vec![departure.clone(), departure2.clone()])
            .await
            .unwrap();
        helper
            .add_ers_por(vec![arrival.clone(), arrival2.clone()])
            .await
            .unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let departure3 = fiskeridir_rs::ErsDep::test_default(3, vessel_id.0 as u64, start3, 3);
        let arrival3 = fiskeridir_rs::ErsPor::test_default(3, vessel_id.0 as u64, end3, 4);

        helper.add_ers_dep(vec![departure3.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival3.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        assert_eq!(
            helper
                .adapter()
                .conflicts(TripAssemblerId::Ers)
                .await
                .unwrap()
                .len(),
            0
        );
    })
    .await;
}

#[tokio::test]
async fn test_ignores_arrivals_and_departures_prior_to_epoch() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(-20, 0).unwrap();
        let end = Utc.timestamp_opt(-10, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure]).await.unwrap();
        helper.add_ers_por(vec![arrival]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        assert_eq!(trips.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_queuing_a_reset_re_creates_trips() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = Utc.timestamp_opt(200000, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].trip_id.0, 1);

        helper.db.queue_trips_reset().await;

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].trip_id.0, 2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_reset_is_cleared_on_next_run() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = Utc.timestamp_opt(200000, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        helper.db.queue_trips_reset().await;

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].trip_id.0, 2);
    })
    .await;
}

#[tokio::test]
async fn test_trips_reset_deletes_all_trips_including_non_overlaps() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = Utc.timestamp_opt(200000, 0).unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 2);

        let departure2 =
            fiskeridir_rs::ErsDep::test_default(2, vessel_id.0 as u64, end + Duration::days(10), 3);
        let arrival2 =
            fiskeridir_rs::ErsPor::test_default(2, vessel_id.0 as u64, end + Duration::days(20), 4);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        helper.db.queue_trips_reset().await;

        helper.add_ers_dep(vec![departure2.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival2.clone()]).await.unwrap();

        ers_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(vessel_id).await;

        assert_eq!(trips.len(), 2);
        assert_eq!(trips[0].trip_id.0, 2);
        assert_eq!(trips[1].trip_id.0, 3);
    })
    .await;
}
