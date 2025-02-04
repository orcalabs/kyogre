use super::helper::{test, test_with_cache};
use chrono::{Duration, TimeZone, Utc};
use engine::*;
use fiskeridir_rs::Gear;
use kyogre_core::{FiskeridirVesselId, ScraperInboundPort, VesselEventType};
use web_api::routes::v1::trip::TripsParameters;

#[tokio::test]
async fn test_trips_does_not_contain_duplicated_tra_events() {
    test_with_cache(|helper, builder| async move {
        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = start + Duration::hours(1);

        let tra = fiskeridir_rs::ErsTra::test_default(
            1,
            Some(FiskeridirVesselId::test_new(1)),
            start + Duration::seconds(1),
        );

        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
            })
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .tra(1)
            .modify(|v| {
                v.tra = tra.clone();
            })
            .build()
            .await;

        helper.db.add_ers_tra(vec![tra]).await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 3);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsTra);
        assert_eq!(trips, state.trips);
    })
    .await;
}

#[tokio::test]
async fn test_trips_does_not_contain_duplicated_dca_events() {
    test_with_cache(|helper, builder| async move {
        let start = Utc.timestamp_opt(100000, 0).unwrap();
        let end = start + Duration::hours(1);

        let mut dca = fiskeridir_rs::ErsDca::test_default(1, Some(FiskeridirVesselId::test_new(1)));
        dca.set_start_timestamp(start + Duration::seconds(1));
        dca.set_stop_timestamp(start + Duration::seconds(2));
        dca.message_info
            .set_message_timestamp(start + Duration::seconds(1));

        let state = builder
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = FiskeridirVesselId::test_new(1);
            })
            .trips(1)
            .modify(|v| {
                v.trip_specification.set_start(start);
                v.trip_specification.set_end(end);
            })
            .hauls(1)
            .modify(|v| {
                v.dca = dca.clone();
            })
            .build()
            .await;

        helper
            .db
            .db
            .add_ers_dca(Box::new(vec![Ok(dca)].into_iter()))
            .await
            .unwrap();

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();
        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 4);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::Haul);
        assert_eq!(trips, state.trips);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_events_connect_to_existing_trip() {
    test_with_cache(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .new_cycle()
            .hauls(2)
            .modify_idx(|idx, v| {
                if idx == 0 {
                    v.dca.gear.gear_fdir_code = Some(Gear::Unknown);
                    v.dca.catch.species.species_fao_code = None;
                    v.dca.catch.species.living_weight = None;
                    v.dca.whale_catch_info.grenade_number = None;
                }
            })
            .tra(1)
            .landings(1)
            .build()
            .await;

        helper.refresh_cache().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 1);
        assert_eq!(trips[0].events.len(), 7);
        assert_eq!(trips[0].events[0].event_type, VesselEventType::ErsDep);
        assert_eq!(trips[0].events[1].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[2].event_type, VesselEventType::ErsDca);
        assert_eq!(trips[0].events[3].event_type, VesselEventType::Haul);
        assert_eq!(trips[0].events[4].event_type, VesselEventType::ErsTra);
        assert_eq!(trips[0].events[5].event_type, VesselEventType::ErsPor);
        assert_eq!(trips[0].events[6].event_type, VesselEventType::Landing);
        assert_eq!(trips, state.trips);
    })
    .await;
}

#[tokio::test]
async fn test_inserting_same_landing_does_not_create_dangling_vessel_event() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .landings(1)
            .modify(|l| l.landing.document_info.version_number = 99)
            .build()
            .await;

        let l = &state.landings[0];
        let mut landing = fiskeridir_rs::Landing::test_default(1, l.fiskeridir_vessel_id);
        landing.id = l.id.clone();

        // We use test builder cycle as data year
        helper
            .db
            .db
            .add_landings(Box::new(vec![Ok(landing)].into_iter()), 1)
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test]
async fn test_inserting_same_ers_dca_does_not_create_dangling_vessel_event() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).build().await;
        let mut dca = fiskeridir_rs::ErsDca::test_default(1, Some(state.vessels[0].fiskeridir.id));

        dca.message_version = 99;
        helper.db.add_ers_dca(vec![dca.clone()]).await;

        dca.message_version = 1;
        helper.db.add_ers_dca(vec![dca]).await;
    })
    .await;
}

#[tokio::test]
async fn test_inserting_same_ers_dep_does_not_create_dangling_vessel_event() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).build().await;
        let dep =
            fiskeridir_rs::ErsDep::test_default(1, state.vessels[0].fiskeridir.id, Utc::now(), 1);

        helper.db.add_ers_departure(vec![dep.clone()]).await;
        helper.db.add_ers_departure(vec![dep]).await;
    })
    .await;
}

#[tokio::test]
async fn test_inserting_same_ers_por_does_not_create_dangling_vessel_event() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).build().await;
        let por =
            fiskeridir_rs::ErsPor::test_default(1, state.vessels[0].fiskeridir.id, Utc::now(), 1);

        helper.db.add_ers_arrival(vec![por.clone()]).await;
        helper.db.add_ers_arrival(vec![por]).await;
    })
    .await;
}

#[tokio::test]
async fn test_inserting_same_ers_tra_does_not_create_dangling_vessel_event() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).build().await;
        let tra = fiskeridir_rs::ErsTra::test_default(
            1,
            Some(state.vessels[0].fiskeridir.id),
            Utc::now(),
        );

        helper.db.add_ers_tra(vec![tra.clone()]).await;
        helper.db.add_ers_tra(vec![tra]).await;
    })
    .await;
}
