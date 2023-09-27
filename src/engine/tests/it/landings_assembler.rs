use crate::helper::test;
use chrono::{Duration, TimeZone, Utc};
use kyogre_core::*;

#[tokio::test]
async fn test_produces_new_trips_without_replacing_existing_ones() {
    test(|_helper, builder| async move {
        let landing = Utc.timestamp_opt(10000000, 0).unwrap();
        let landing2 = Utc.timestamp_opt(30000000, 0).unwrap();
        let state = builder
            .vessels(1)
            .landings(1)
            .modify(|v| v.landing.landing_timestamp = landing)
            .new_cycle()
            .landings(1)
            .modify(|v| v.landing.landing_timestamp = landing2)
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        assert_eq!(
            state.trips[0].period.start(),
            state.landings[0].landing_timestamp - Duration::days(1)
        );
        assert_eq!(
            state.trips[0].period.end(),
            state.landings[0].landing_timestamp
        );
        assert_eq!(state.trips[0].period, state.trips[0].landing_coverage);
        assert_eq!(
            state.trips[1].period.start(),
            state.landings[0].landing_timestamp
        );
        assert_eq!(
            state.trips[1].period.end(),
            state.landings[1].landing_timestamp
        );
        assert_eq!(state.trips[1].period, state.trips[1].landing_coverage);
    })
    .await;
}

#[tokio::test]
async fn test_produces_no_trips_with_no_new_landings() {
    test(|_helper, builder| async move {
        let state = builder.vessels(1).landings(1).new_cycle().build().await;

        assert_eq!(state.trips.len(), 1);
        assert_eq!(
            state.trips[0].period.start(),
            state.landings[0].landing_timestamp - Duration::days(1)
        );
        assert_eq!(
            state.trips[0].period.end(),
            state.landings[0].landing_timestamp
        );
        assert_eq!(state.trips[0].period, state.trips[0].landing_coverage);
    })
    .await;
}

#[tokio::test]
async fn test_resolves_conflict_on_day_prior_to_most_recent_trip_end() {
    test(|_helper, builder| async move {
        let landing = Utc.timestamp_opt(10000000, 0).unwrap();
        let landing2 = Utc.timestamp_opt(30000000, 0).unwrap();
        let landing3 = Utc.timestamp_opt(20000000, 0).unwrap();
        let state = builder
            .vessels(1)
            .landings(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.landing.landing_timestamp = landing;
                } else {
                    v.landing.landing_timestamp = landing2;
                }
            })
            .new_cycle()
            .landings(1)
            .modify(|v| v.landing.landing_timestamp = landing3)
            .build()
            .await;

        assert_eq!(state.trips.len(), 3);
        assert_eq!(
            state.trips[0].period.start(),
            state.landings[0].landing_timestamp - Duration::days(1)
        );
        assert_eq!(state.trips[0].period.end(), landing,);
        assert_eq!(state.trips[0].period, state.trips[0].landing_coverage);
        assert_eq!(state.trips[1].period.start(), landing);
        assert_eq!(state.trips[1].period.end(), landing3);
        assert_eq!(state.trips[1].period, state.trips[1].landing_coverage);
        assert_eq!(state.trips[2].period.start(), landing3);
        assert_eq!(state.trips[2].period.end(), landing2);
        assert_eq!(state.trips[2].period, state.trips[2].landing_coverage);
    })
    .await;
}

#[tokio::test]
async fn test_other_event_types_does_not_cause_conflicts() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .landings(1)
            .dep(1)
            .hauls(1)
            .tra(1)
            .por(1)
            .landings(1)
            .build()
            .await;

        assert!(helper
            .adapter()
            .conflict(state.vessels[0].fiskeridir.id, TripAssemblerId::Landings)
            .await
            .unwrap()
            .is_none());
    })
    .await;
}

// // // TODO: figure out of we want to support this case
// // // #[tokio::test]
// // // async fn test_landing_later_on_same_day_as_first_trip_causes_conflict_and_results_in_trip_extension(
// // // ) {
// // //     test(|helper| async move {
// // //         let fiskeridir_vessel_id = 1;
// // //         let landings_assembler = LandingTripAssembler::default();
// // //         let landing = fiskeridir_rs::Landing::test_default(1, Some(fiskeridir_vessel_id));
// // //         helper.add_landings(vec![landing.clone()]).await.unwrap();

// // //         let first_landing_timestamp = landing.landing_timestamp;

// // //         let vessel = helper.db.vessel(fiskeridir_vessel_id).await;
// // //         let assembled = landings_assembler
// // //             .assemble(&helper.db.db, &vessel, State::NoPriorState)
// // //             .await
// // //             .unwrap()
// // //             .unwrap();

// // //         helper
// // //             .add_trips(
// // //                 vessel.fiskeridir.id,
// // //                 assembled.new_trip_calucation_time,
// // //                 assembled.conflict_strategy,
// // //                 assembled.trips,
// // //                 TripAssemblerId::Landings,
// // //             )
// // //             .await
// // //             .unwrap();

// // //         let mut landing = fiskeridir_rs::Landing::test_default(2, Some(fiskeridir_vessel_id));
// // //         landing.landing_timestamp = DateTime::<Utc>::from_utc(
// // //             NaiveDateTime::new(
// // //                 landing.landing_timestamp.date_naive(),
// // //                 NaiveTime::from_hms_opt(23, 50, 50).unwrap(),
// // //             ),
// // //             Utc,
// // //         );
// // //         helper.add_landings(vec![landing.clone()]).await.unwrap();

// // //         let conflict = helper
// // //             .conflicts(TripAssemblerId::Landings)
// // //             .await
// // //             .unwrap()
// // //             .pop()
// // //             .unwrap();

// // //         dbg!("SECOND RUN");
// // //         let assembled = landings_assembler
// // //             .assemble(
// // //                 &helper.db.db,
// // //                 &vessel,
// // //                 State::Conflict {
// // //                     conflict_timestamp: conflict.timestamp,
// // //                     trip_prior_to_or_at_conflict: helper
// // //                         .trip_at_or_prior_to(
// // //                             vessel.fiskeridir.id,
// // //                             TripAssemblerId::Landings,
// // //                             &conflict.timestamp,
// // //                         )
// // //                         .await
// // //                         .unwrap(),
// // //                 },
// // //             )
// // //             .await
// // //             .unwrap()
// // //             .unwrap();
// // //         helper
// // //             .add_trips(
// // //                 vessel.fiskeridir.id,
// // //                 assembled.new_trip_calucation_time,
// // //                 assembled.conflict_strategy,
// // //                 assembled.trips,
// // //                 TripAssemblerId::Landings,
// // //             )
// // //             .await
// // //             .unwrap();

// // //         let trips = helper.db.trips_of_vessel(vessel.fiskeridir.id).await;
// // //         assert_eq!(1, trips.len());

// // //         let expected_range = DateRange::new(
// // //             first_landing_timestamp - Duration::days(1),
// // //             landing.landing_timestamp,
// // //         )
// // //         .unwrap();

// // //         let expected = Trip {
// // //             trip_id: 2,
// // //             range: expected_range.clone(),
// // //             landing_coverage: expected_range,
// // //             assembler_id: TripAssemblerId::Landings,
// // //         };
// // //         assert_eq!(expected, trips[0]);
// // //     })
// // //     .await;
// // // }
