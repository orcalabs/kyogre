use crate::helper::test;
use chrono::{Duration, TimeZone, Utc};
use kyogre_core::{
    DateRange, FiskeridirVesselId, ScraperInboundPort, Trip, TripAssemblerId,
    TripAssemblerOutboundPort, TripId,
};
use trip_assembler::{LandingTripAssembler, TripAssembler};

#[tokio::test]
async fn test_produces_new_trips_without_replacing_existing_ones() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let landings_assembler = LandingTripAssembler::default();

        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        helper
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id.0));
        landing2.landing_timestamp = landing.landing_timestamp + Duration::days(1);
        helper
            .add_landings(vec![landing2.clone()], 2023)
            .await
            .unwrap();

        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let expected_range_1 = DateRange::new(
            landing.landing_timestamp - Duration::days(1),
            landing.landing_timestamp,
        )
        .unwrap();

        let mut trips = helper.db.trips_of_vessel(vessel.fiskeridir.id).await;
        trips.sort_by_key(|v| v.trip_id);

        let expected_range_2 =
            DateRange::new(landing.landing_timestamp, landing2.landing_timestamp).unwrap();

        let expected = vec![
            Trip {
                trip_id: TripId(1),
                period: expected_range_1.clone(),
                landing_coverage: expected_range_1,
                assembler_id: TripAssemblerId::Landings,
                precision_period: None,
                distance: None,
            },
            Trip {
                trip_id: TripId(2),
                period: expected_range_2.clone(),
                landing_coverage: expected_range_2,
                assembler_id: TripAssemblerId::Landings,
                precision_period: None,
                distance: None,
            },
        ];
        assert_eq!(expected, trips);
    })
    .await;
}

#[tokio::test]
async fn test_produces_no_trips_with_no_new_landings() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let landings_assembler = LandingTripAssembler::default();

        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        helper
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let expected_range = DateRange::new(
            landing.landing_timestamp - Duration::days(1),
            landing.landing_timestamp,
        )
        .unwrap();

        let trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;
        assert_eq!(trips.len(), 1);

        let expected = Trip {
            trip_id: TripId(1),
            period: expected_range.clone(),
            landing_coverage: expected_range,
            assembler_id: TripAssemblerId::Landings,
            precision_period: None,
            distance: None,
        };

        assert_eq!(expected, trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_sets_start_of_first_trip_one_day_earlier_than_landing_timestamp() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let landings_assembler = LandingTripAssembler::default();

        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        helper
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;
        assert_eq!(trips.len(), 1);

        let expected_range = DateRange::new(
            landing.landing_timestamp - Duration::days(1),
            landing.landing_timestamp,
        )
        .unwrap();

        let expected = Trip {
            trip_id: TripId(1),
            period: expected_range.clone(),
            landing_coverage: expected_range,
            assembler_id: TripAssemblerId::Landings,
            precision_period: None,
            distance: None,
        };
        assert_eq!(expected, trips[0]);
    })
    .await;
}

#[tokio::test]
async fn test_resolves_conflict_on_day_prior_to_most_recent_trip_end() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let landings_assembler = LandingTripAssembler::default();
        let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        let mut landing2 = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id.0));
        landing2.landing_timestamp += Duration::days(3);

        helper
            .add_landings(vec![landing.clone(), landing2.clone()], 2023)
            .await
            .unwrap();

        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut landing3 = fiskeridir_rs::Landing::test_default(3, Some(fiskeridir_vessel_id.0));
        landing3.landing_timestamp = landing2.landing_timestamp - Duration::days(1);

        helper
            .add_landings(vec![landing3.clone()], 2023)
            .await
            .unwrap();

        landings_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let mut trips = helper.db.trips_of_vessel(fiskeridir_vessel_id).await;
        assert_eq!(3, trips.len());
        trips.sort_by_key(|v| v.trip_id);

        let expected_range_1 = DateRange::new(
            landing.landing_timestamp - Duration::days(1),
            landing.landing_timestamp,
        )
        .unwrap();

        let expected_range_2 =
            DateRange::new(landing.landing_timestamp, landing3.landing_timestamp).unwrap();

        let expected_range_3 =
            DateRange::new(landing3.landing_timestamp, landing2.landing_timestamp).unwrap();

        let expected = vec![
            Trip {
                trip_id: TripId(1),
                period: expected_range_1.clone(),
                landing_coverage: expected_range_1,
                assembler_id: TripAssemblerId::Landings,
                precision_period: None,
                distance: None,
            },
            Trip {
                trip_id: TripId(3),
                period: expected_range_2.clone(),
                landing_coverage: expected_range_2,
                assembler_id: TripAssemblerId::Landings,
                precision_period: None,
                distance: None,
            },
            Trip {
                // One trip is deleted so serial key is incremented
                trip_id: TripId(4),
                period: expected_range_3.clone(),
                landing_coverage: expected_range_3,
                assembler_id: TripAssemblerId::Landings,
                precision_period: None,
                distance: None,
            },
        ];
        assert_eq!(expected, trips);
    })
    .await;
}
#[tokio::test]
async fn test_other_event_types_does_not_cause_conflicts() {
    test(|helper| async move {
        let adapter = helper.adapter();
        let vessel_id = FiskeridirVesselId(11);
        let landing_assembler = LandingTripAssembler::default();

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        helper.db.generate_landing(1, vessel_id, start).await;
        helper.db.generate_landing(1, vessel_id, end).await;

        landing_assembler
            .produce_and_store_trips(adapter)
            .await
            .unwrap();

        let departure = fiskeridir_rs::ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        let arrival = fiskeridir_rs::ErsPor::test_default(1, vessel_id.0 as u64, end, 1);

        helper.add_ers_dep(vec![departure.clone()]).await.unwrap();
        helper.add_ers_por(vec![arrival.clone()]).await.unwrap();

        helper.db.generate_tra(1, vessel_id, start).await;
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

// // TODO: figure out of we want to support this case
// // #[tokio::test]
// // async fn test_landing_later_on_same_day_as_first_trip_causes_conflict_and_results_in_trip_extension(
// // ) {
// //     test(|helper| async move {
// //         let fiskeridir_vessel_id = 1;
// //         let landings_assembler = LandingTripAssembler::default();
// //         let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id));
// //         helper.add_landings(vec![landing.clone()]).await.unwrap();

// //         let first_landing_timestamp = landing.landing_timestamp;

// //         let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
// //         let assembled = landings_assembler
// //             .assemble(&helper.db.db, &vessel, State::NoPriorState)
// //             .await
// //             .unwrap()
// //             .unwrap();

// //         helper
// //             .add_trips(
// //                 vessel.fiskeridir.id,
// //                 assembled.new_trip_calucation_time,
// //                 assembled.conflict_strategy,
// //                 assembled.trips,
// //                 TripAssemblerId::Landings,
// //             )
// //             .await
// //             .unwrap();

// //         let mut landing = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id));
// //         landing.landing_timestamp = DateTime::<Utc>::from_utc(
// //             NaiveDateTime::new(
// //                 landing.landing_timestamp.date_naive(),
// //                 NaiveTime::from_hms_opt(23, 50, 50).unwrap(),
// //             ),
// //             Utc,
// //         );
// //         helper.add_landings(vec![landing.clone()]).await.unwrap();

// //         let conflict = helper
// //             .conflicts(TripAssemblerId::Landings)
// //             .await
// //             .unwrap()
// //             .pop()
// //             .unwrap();

// //         dbg!("SECOND RUN");
// //         let assembled = landings_assembler
// //             .assemble(
// //                 &helper.db.db,
// //                 &vessel,
// //                 State::Conflict {
// //                     conflict_timestamp: conflict.timestamp,
// //                     trip_prior_to_or_at_conflict: helper
// //                         .trip_at_or_prior_to(
// //                             vessel.fiskeridir.id,
// //                             TripAssemblerId::Landings,
// //                             &conflict.timestamp,
// //                         )
// //                         .await
// //                         .unwrap(),
// //                 },
// //             )
// //             .await
// //             .unwrap()
// //             .unwrap();
// //         helper
// //             .add_trips(
// //                 vessel.fiskeridir.id,
// //                 assembled.new_trip_calucation_time,
// //                 assembled.conflict_strategy,
// //                 assembled.trips,
// //                 TripAssemblerId::Landings,
// //             )
// //             .await
// //             .unwrap();

// //         let trips = helper.db.trips_of_vessel(vessel.fiskeridir.id).await;
// //         assert_eq!(1, trips.len());

// //         let expected_range = DateRange::new(
// //             first_landing_timestamp - Duration::days(1),
// //             landing.landing_timestamp,
// //         )
// //         .unwrap();

// //         let expected = Trip {
// //             trip_id: 2,
// //             range: expected_range.clone(),
// //             landing_coverage: expected_range,
// //             assembler_id: TripAssemblerId::Landings,
// //         };
// //         assert_eq!(expected, trips[0]);
// //     })
// //     .await;
// // }
