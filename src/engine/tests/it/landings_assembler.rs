use crate::helper::test;
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use kyogre_core::*;

#[tokio::test]
async fn test_does_not_logs_actions_on_success() {
    test(|helper, builder| async move {
        let timestamp = Utc.timestamp_opt(1000000000, 0).unwrap();
        builder
            .vessels(1)
            .landings(1)
            .modify(|v| {
                v.landing.landing_timestamp = timestamp;
            })
            .landings(1)
            .modify(|v| {
                v.landing.landing_timestamp = timestamp + Duration::days(1);
            })
            .build()
            .await;

        let logs = helper.adapter().trip_assembler_log().await;
        assert!(logs.is_empty());
    })
    .await
}
#[tokio::test]
async fn test_produces_new_trips_without_replacing_existing_ones() {
    test(|_helper, builder| async move {
        let landing = Utc.timestamp_opt(10000000, 0).unwrap();
        let landing2 = landing + Duration::days(14);

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
        let landing2 = landing + Duration::days(14);
        let landing3 = landing + Duration::days(7);

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
        assert_eq!(state.trips[0].period.end(), landing);
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
            .hauls(1)
            .tra(1)
            .por(1)
            .landings(1)
            .build()
            .await;

        assert!(helper
            .adapter()
            .trip_calculation_timer(state.vessels[0].fiskeridir.id, TripAssemblerId::Landings)
            .await
            .unwrap()
            .unwrap()
            .conflict
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn test_resolves_conflict_on_same_day_as_most_recent_trip_end() {
    test(|_helper, builder| async move {
        let landing = Utc.timestamp_opt(10000000, 0).unwrap();
        let landing2 = landing + Duration::days(14);
        let landing3 = landing2 + Duration::seconds(1);

        let state = builder
            .vessels(1)
            .landings(2)
            .modify_idx(|i, v| {
                if i == 0 {
                    v.landing.landing_timestamp = landing;
                } else if i == 1 {
                    v.landing.landing_timestamp = landing2;
                }
            })
            .new_cycle()
            .landings(1)
            .modify(|v| v.landing.landing_timestamp = landing3)
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
        assert_eq!(
            state.trips[0].period.start(),
            state.landings[0].landing_timestamp - Duration::days(1)
        );
        assert_eq!(state.trips[0].period.end(), landing);
        assert_eq!(state.trips[0].period, state.trips[0].landing_coverage);
        assert_eq!(state.trips[1].period.start(), landing);
        assert_eq!(state.trips[1].period.end(), landing3);
        assert_eq!(state.trips[1].period, state.trips[1].landing_coverage);
    })
    .await;
}

#[tokio::test]
async fn test_deleting_landing_deletes_corresponding_trip_with_no_prior_trip() {
    test(|helper, builder| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        let landing_ids = [
            "2000-01-24T00:00:00Z".parse().unwrap(),
            "2000-01-25T00:00:00Z".parse().unwrap(),
            "2000-01-27T00:00:00Z".parse().unwrap(),
            "2000-01-29T00:00:00Z".parse().unwrap(),
        ];

        let landings = landing_ids
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut l = fiskeridir_rs::Landing::test_default(i as _, Some(vessel_id));
                l.landing_timestamp = *v;
                l
            })
            .collect::<Vec<_>>();

        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id;
            })
            .landings(3)
            .modify_idx(|i, v| {
                v.landing = landings[i].clone();
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 3);

        let state = helper
            .builder()
            .await
            .landings(2)
            .modify_idx(|i, v| {
                v.landing = landings[i].clone();
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_deleting_landing_deletes_corresponding_trip_with_prior_trip() {
    test(|helper, builder| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        let landing_ids = [
            "2000-01-23T00:00:00Z".parse().unwrap(),
            "2000-01-24T00:00:00Z".parse().unwrap(),
            "2000-01-25T00:00:00Z".parse().unwrap(),
            "2000-01-27T00:00:00Z".parse().unwrap(),
            "2000-01-29T00:00:00Z".parse().unwrap(),
        ];

        let landings = landing_ids
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut l = fiskeridir_rs::Landing::test_default(i as _, Some(vessel_id));
                l.landing_timestamp = *v;
                l
            })
            .collect::<Vec<_>>();

        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id;
            })
            .landings(4)
            .modify_idx(|i, v| {
                v.landing = landings[i].clone();
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 4);

        let state = helper
            .builder()
            .await
            .landings(3)
            .modify_idx(|i, v| {
                v.landing = landings[i].clone();
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_deleting_only_landing_deletes_trip() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).landings(1).build().await;
        assert_eq!(state.trips.len(), 1);

        let state = helper.builder().await.build().await;
        assert_eq!(state.trips.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_deleting_second_landing_deletes_trip() {
    test(|helper, builder| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        let landing_ids = [
            "2000-01-24T00:00:00Z".parse().unwrap(),
            "2000-01-25T00:00:00Z".parse().unwrap(),
        ];

        let landings = landing_ids
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut l = fiskeridir_rs::Landing::test_default(i as _, Some(vessel_id));
                l.landing_timestamp = *v;
                l
            })
            .collect::<Vec<_>>();

        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id;
            })
            .landings(2)
            .modify_idx(|i, v| {
                v.landing = landings[i].clone();
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 2);

        let state = helper
            .builder()
            .await
            .landings(1)
            .modify_idx(|i, v| {
                v.landing = landings[i].clone();
            })
            .build()
            .await;

        assert_eq!(state.trips.len(), 1);
    })
    .await;
}
