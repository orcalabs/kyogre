use crate::helper::*;
use chrono::Duration;
use kyogre_core::*;
use trip_assembler::*;

#[tokio::test]
async fn test_produces_new_trips_without_replacing_existing_ones() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let departure = fiskeridir_rs::ErsDep::test_default(1, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival =
            fiskeridir_rs::ErsPor::test_default(1, Some(fiskeridir_vessel_id.0 as u64), true);
        arrival.arrival_date = departure.departure_date + Duration::days(1);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
        let assembled = ers_assembler
            .assemble(&helper.db.db, &vessel, State::NoPriorState)
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        let mut departure2 =
            fiskeridir_rs::ErsDep::test_default(2, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival2 =
            fiskeridir_rs::ErsPor::test_default(2, Some(fiskeridir_vessel_id.0 as u64), true);
        departure2.departure_date = departure.departure_date + Duration::days(2);
        arrival2.arrival_date = departure2.departure_date + Duration::days(3);

        helper.add_ers_dep(vec![departure2.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival2.clone()]).await.unwrap();

        let assembled = ers_assembler
            .assemble(
                &helper.db.db,
                &vessel,
                State::CurrentCalculationTime(assembled.new_trip_calculation_time),
            )
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel.fiskeridir.id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected = vec![
            Trip {
                trip_id: TripId(1),
                period: create_date_range(&departure, &arrival),
                landing_coverage: create_date_range(&departure, &departure2),
                assembler_id: TripAssemblerId::Ers,
            },
            Trip {
                trip_id: TripId(2),
                period: create_date_range(&departure2, &arrival2),
                landing_coverage: create_date_range(
                    &departure2,
                    &ers_last_trip_landing_coverage_end(),
                ),
                assembler_id: TripAssemblerId::Ers,
            },
        ];
        assert_eq!(expected, trips);
    })
    .await;
}

#[tokio::test]
async fn test_produces_no_trips_with_no_new_departures_or_arrivals() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let departure = fiskeridir_rs::ErsDep::test_default(1, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival =
            fiskeridir_rs::ErsPor::test_default(1, Some(fiskeridir_vessel_id.0 as u64), true);
        arrival.arrival_date = departure.departure_date + Duration::days(1);
        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
        let assembled = ers_assembler
            .assemble(&helper.db.db, &vessel, State::NoPriorState)
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        assert!(ers_assembler
            .assemble(
                &helper.db.db,
                &vessel,
                State::CurrentCalculationTime(assembled.new_trip_calculation_time)
            )
            .await
            .unwrap()
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn test_extends_most_recent_trip_with_new_arrival() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let departure = fiskeridir_rs::ErsDep::test_default(1, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival =
            fiskeridir_rs::ErsPor::test_default(1, Some(fiskeridir_vessel_id.0 as u64), true);
        arrival.arrival_date = departure.departure_date + Duration::days(1);
        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
        let assembled = ers_assembler
            .assemble(&helper.db.db, &vessel, State::NoPriorState)
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        let mut arrival2 =
            fiskeridir_rs::ErsPor::test_default(2, Some(fiskeridir_vessel_id.0 as u64), true);
        arrival2.arrival_date = departure.departure_date + Duration::days(2);

        helper.add_ers_por(vec![arrival2.clone()]).await.unwrap();

        let conflict = helper
            .conflicts(TripAssemblerId::Ers)
            .await
            .unwrap()
            .pop()
            .unwrap();

        let assembled = ers_assembler
            .assemble(
                &helper.db.db,
                &vessel,
                State::Conflict {
                    conflict_timestamp: conflict.timestamp,
                    trip_prior_to_or_at_conflict: helper
                        .trip_at_or_prior_to(
                            vessel.fiskeridir.id,
                            TripAssemblerId::Ers,
                            &conflict.timestamp,
                        )
                        .await
                        .unwrap(),
                },
            )
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel.fiskeridir.id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected = vec![Trip {
            trip_id: TripId(2),
            period: create_date_range(&departure, &arrival2),
            landing_coverage: create_date_range(&departure, &ers_last_trip_landing_coverage_end()),
            assembler_id: TripAssemblerId::Ers,
        }];
        assert_eq!(expected, trips);
    })
    .await;
}

#[tokio::test]
async fn test_handles_conflict_correctly() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let ers_assembler = ErsTripAssembler::default();

        let departure = fiskeridir_rs::ErsDep::test_default(1, Some(fiskeridir_vessel_id.0 as u64));
        let base_time = departure.departure_date;
        let mut arrival =
            fiskeridir_rs::ErsPor::test_default(1, Some(fiskeridir_vessel_id.0 as u64), true);
        arrival.arrival_date = base_time + Duration::days(1);

        let mut departure2 =
            fiskeridir_rs::ErsDep::test_default(2, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival2 =
            fiskeridir_rs::ErsPor::test_default(2, Some(fiskeridir_vessel_id.0 as u64), true);
        departure2.departure_date = base_time + Duration::days(5);
        arrival2.arrival_date = base_time + Duration::days(6);

        helper
            .add_ers_dep(vec![departure.clone(), departure2.clone()])
            .await
            .unwrap();
        helper
            .add_ers_por(vec![arrival.clone(), arrival2.clone()])
            .await
            .unwrap();

        let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
        let assembled = ers_assembler
            .assemble(&helper.db.db, &vessel, State::NoPriorState)
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        let mut departure3 =
            fiskeridir_rs::ErsDep::test_default(3, Some(fiskeridir_vessel_id.0 as u64));
        let mut arrival3 =
            fiskeridir_rs::ErsPor::test_default(3, Some(fiskeridir_vessel_id.0 as u64), true);
        departure3.departure_date = base_time + Duration::days(3);
        arrival3.arrival_date = base_time + Duration::days(4);

        helper.add_ers_dep(vec![departure3.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival3.clone()]).await.unwrap();

        let conflict = helper
            .conflicts(TripAssemblerId::Ers)
            .await
            .unwrap()
            .pop()
            .unwrap();

        let assembled = ers_assembler
            .assemble(
                &helper.db.db,
                &vessel,
                State::Conflict {
                    conflict_timestamp: conflict.timestamp,
                    trip_prior_to_or_at_conflict: helper
                        .trip_at_or_prior_to(
                            vessel.fiskeridir.id,
                            TripAssemblerId::Ers,
                            &conflict.timestamp,
                        )
                        .await
                        .unwrap(),
                },
            )
            .await
            .unwrap()
            .unwrap();

        helper
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                TripAssemblerId::Ers,
            )
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel.fiskeridir.id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected = vec![
            Trip {
                trip_id: TripId(3),
                period: create_date_range(&departure, &arrival),
                landing_coverage: create_date_range(&departure, &departure3),
                assembler_id: TripAssemblerId::Ers,
            },
            Trip {
                trip_id: TripId(4),
                period: create_date_range(&departure3, &arrival3),
                landing_coverage: create_date_range(&departure3, &departure2),
                assembler_id: TripAssemblerId::Ers,
            },
            Trip {
                trip_id: TripId(5),
                period: create_date_range(&departure2, &arrival2),
                landing_coverage: create_date_range(
                    &departure2,
                    &ers_last_trip_landing_coverage_end(),
                ),
                assembler_id: TripAssemblerId::Ers,
            },
        ];
        assert_eq!(expected, trips);
    })
    .await;
}