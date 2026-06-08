use super::helper::test;
use chrono::{Duration, Utc};
use engine::*;
use fiskeridir_rs::Gear;
use http_client::StatusCode;
use kyogre_core::{HaulEnd, HaulStart, UpdateUserHaul, UserHaulId};
use serde_json::json;
use web_api::{error::ErrorDiscriminants, routes::v1::trip::TripsParameters};

#[tokio::test]
async fn test_get_user_hauls() {
    test(|mut helper, builder| async move {
        let state = builder.user_hauls(3).build().await;
        helper.app.login_user_with_id(state.user_id);

        let user_hauls = helper.app.user_hauls().await.unwrap();

        assert_eq!(user_hauls.len(), 3);
        assert_eq!(user_hauls, state.user_hauls)
    })
    .await;
}

#[tokio::test]
async fn test_delete_user_hauls() {
    test(|mut helper, builder| async move {
        let state = builder.user_hauls(3).build().await;
        helper.app.login_user_with_id(state.user_id);

        let id = state.user_hauls[0].id;

        helper.app.delete_user_haul(id).await.unwrap();

        let user_hauls = helper.app.user_hauls().await.unwrap();

        assert_eq!(user_hauls.len(), 2);
        assert!(!user_hauls.iter().any(|h| h.id == id))
    })
    .await;
}

#[tokio::test]
async fn test_delete_user_hauls_fails_on_non_existing_id() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        let error = helper
            .app
            .delete_user_haul(UserHaulId::test_new())
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error, ErrorDiscriminants::ObjectNotFound);
    })
    .await;
}

#[tokio::test]
async fn test_start_and_current_user_haul() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        let start = HaulStart::test_default();

        let pre_ts = Utc::now();
        let started_haul = helper.app.start_user_haul(&start).await.unwrap();
        let current = helper.app.current_user_haul().await.unwrap().unwrap();

        assert!(started_haul.start_ts < Utc::now());
        assert!(started_haul.start_ts > pre_ts);
        assert_eq!(start, started_haul);
        assert_eq!(current, started_haul);
    })
    .await;
}

#[tokio::test]
async fn test_abort_user_haul() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        helper
            .app
            .start_user_haul(&HaulStart::test_default())
            .await
            .unwrap();
        helper.app.abort_user_haul().await.unwrap();

        let user_hauls = helper.app.user_hauls().await.unwrap();
        let current = helper.app.current_user_haul().await.unwrap();
        assert!(user_hauls.is_empty());
        assert!(current.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_abort_user_haul_fails_without_any_current_active_hauls() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;
        helper.app.login_user_with_id(state.user_id);

        let error = helper.app.abort_user_haul().await.unwrap_err();

        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.error, ErrorDiscriminants::NoActiveUserHaul);
    })
    .await;
}

#[tokio::test]
async fn test_stop_user_haul() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;
        helper.app.login_user_with_id(state.user_id);

        let end = HaulEnd::test_default();
        let start = HaulStart::test_default();

        helper.app.start_user_haul(&start).await.unwrap();
        let finished_haul = helper.app.stop_user_haul(&end).await.unwrap();

        assert_eq!(finished_haul, start);
        assert_eq!(finished_haul, end);
    })
    .await;
}

#[tokio::test]
async fn test_stop_user_haul_fails_without_any_current_active_hauls() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;
        helper.app.login_user_with_id(state.user_id);

        let error = helper
            .app
            .stop_user_haul(&HaulEnd::test_default())
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.error, ErrorDiscriminants::NoActiveUserHaul);
    })
    .await;
}

#[tokio::test]
async fn test_update_user_haul() {
    test(|mut helper, builder| async move {
        let state = builder.user_hauls(1).build().await;

        helper.app.login_user_with_id(state.user_id);

        let update = UpdateUserHaul::test_default();
        let updated = helper
            .app
            .update_user_haul(state.user_hauls[0].id, &update)
            .await
            .unwrap();

        assert_eq!(updated, update);
    })
    .await;
}

#[tokio::test]
async fn test_update_user_haul_fails_on_non_existing_id() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        let error = helper
            .app
            .update_user_haul(UserHaulId::test_new(), &UpdateUserHaul::test_default())
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.error, ErrorDiscriminants::ObjectNotFound);
    })
    .await;
}

#[tokio::test]
async fn test_update_current_user_haul() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        helper
            .app
            .start_user_haul(&HaulStart::test_default())
            .await
            .unwrap();

        let update = HaulStart {
            fuel_liter_start: 1,
            config: json!("the_fuck: 27"),
            gear: Gear::PurseSeine,
        };
        let updated_current = helper.app.update_current_user_haul(&update).await.unwrap();

        let current = helper.app.current_user_haul().await.unwrap().unwrap();

        assert_eq!(updated_current, current);
        assert_eq!(update, updated_current);
    })
    .await;
}

#[tokio::test]
async fn test_update_current_user_haul_fails_without_any_active_hauls() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        let error = helper
            .app
            .update_current_user_haul(&HaulStart::test_default())
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.error, ErrorDiscriminants::NoActiveUserHaul);
    })
    .await;
}

#[tokio::test]
async fn test_update_current_active_haul_through_update_user_haul_fails() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        let current = helper
            .app
            .start_user_haul(&HaulStart::test_default())
            .await
            .unwrap();
        let error = helper
            .app
            .update_user_haul(current.id, &UpdateUserHaul::test_default())
            .await
            .unwrap_err();

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::CannotModifyActiveUserHaul);
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_are_connected_to_trips_and_uses_ers_start_and_stop() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .user_hauls()
            .modify(|h| {
                h.start_ts += Duration::seconds(1);
                h.end_ts -= Duration::seconds(1);
            })
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(trip.hauls[0], state.user_hauls[0]);
        assert_eq!(
            trip.hauls[0].start_timestamp,
            state.hauls[0].start_timestamp
        );
        assert_eq!(trip.hauls[0].stop_timestamp, state.hauls[0].stop_timestamp);

        assert_ne!(trip.hauls[0].start_timestamp, state.user_hauls[0].start_ts);
        assert_ne!(trip.hauls[0].stop_timestamp, state.user_hauls[0].end_ts);
    })
    .await;
}

#[tokio::test]
async fn test_haul_connection_is_not_set_when_there_are_multiple_overlapping_user_hauls_on_a_single_haul()
 {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .overlapping_user_hauls(2)
            .modify_idx(|i, h| {
                if i == 0 {
                    h.end_ts = h.start_ts + DEFAULT_HAUL_DURATION / 4;
                    h.start_ts -= DEFAULT_HAUL_DURATION;
                } else {
                    h.start_ts += DEFAULT_HAUL_DURATION / 3;
                }
            })
            .build()
            .await;

        helper.run_engine_cycle().await;
        helper.app.login_user_with_id(state.user_id);

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(trip.hauls[0].id.is_none());
        assert!(trip.hauls[0].trip_id.is_some());

        assert!(trip.hauls[1].id.is_some());
        assert!(trip.hauls[1].trip_id.is_some());

        assert!(trip.hauls[2].id.is_none());
        assert!(trip.hauls[2].trip_id.is_some());
    })
    .await;
}

#[tokio::test]
async fn test_haul_connection_is_not_set_when_there_are_multiple_overlapping_hauls_on_a_single_user_haul()
 {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(2)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper
            .app
            .start_user_haul(&HaulStart::test_default())
            .await
            .unwrap();
        let haul = helper
            .app
            .stop_user_haul(&HaulEnd::test_default())
            .await
            .unwrap();

        let mut update = UpdateUserHaul::test_default();
        update.start_ts = state.hauls[0].start_timestamp;
        update.end_ts = state.hauls[1].stop_timestamp;
        helper.app.update_user_haul(haul.id, &update).await.unwrap();

        helper.run_processors().await;
        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(!trip.hauls[0].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_trips_are_updated_on_user_haul_deletion() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .user_hauls()
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper
            .app
            .delete_user_haul(state.user_hauls[0].id)
            .await
            .unwrap();

        helper.run_processors().await;
        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(!trip.hauls[0].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_trips_are_updated_on_user_haul_update_when_user_haul_is_no_longer_overlapping_with_a_haul()
 {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .user_hauls()
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        let mut update = UpdateUserHaul::test_default();
        update.start_ts = state.trips[0].period.end() - Duration::seconds(10);
        update.end_ts = state.trips[0].period.end();
        helper
            .app
            .update_user_haul(state.user_hauls[0].id, &update)
            .await
            .unwrap();

        helper.run_processors().await;
        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(!trip.hauls[0].is_user_haul());
        assert!(trip.hauls[1].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_trips_are_updated_on_user_haul_update_when_user_haul_was_moved_to_another_overlapping_haul()
 {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .hauls(1)
            .user_hauls()
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        let mut update = UpdateUserHaul::test_default();
        update.start_ts = state.hauls[0].start_timestamp;
        update.end_ts = state.hauls[0].stop_timestamp;

        helper
            .app
            .update_user_haul(state.user_hauls[0].id, &update)
            .await
            .unwrap();

        helper.run_processors().await;
        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(state.user_hauls.len(), 1);
        assert!(trip.hauls[0].is_user_haul());
        assert!(!trip.hauls[1].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_trips_are_updated_on_user_haul_data_update() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .user_hauls()
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        let mut update = UpdateUserHaul::test_default();
        update.start_ts = state.hauls[0].start_timestamp;
        update.end_ts = state.hauls[0].stop_timestamp;
        update.start_fuel_liter = 10;
        update.end_fuel_liter = 8;
        update.config = json!("jaman");

        helper
            .app
            .update_user_haul(state.user_hauls[0].id, &update)
            .await
            .unwrap();

        helper.run_processors().await;
        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(trip.hauls[0].is_user_haul());
        assert_eq!(*trip.hauls[0].config.as_ref().unwrap(), update.config);
        assert_eq!(
            trip.hauls[0].start_fuel_liter.unwrap(),
            update.start_fuel_liter
        );
        assert_eq!(trip.hauls[0].end_fuel_liter.unwrap(), update.end_fuel_liter);
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_are_added_to_trip_even_if_there_are_no_overlapping_hauls() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .user_hauls(2)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(trip.hauls.len(), 2);
        assert!(trip.hauls[0].is_user_haul());
        assert!(trip.hauls[1].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_mixes_with_hauls_on_trip() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(1)
            .up()
            .user_hauls(1)
            .hauls(1)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(trip.hauls.len(), 3);
        assert!(!trip.hauls[0].is_user_haul());
        assert!(trip.hauls[1].is_user_haul());
        assert!(!trip.hauls[2].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_mixes_with_other_data_on_trip() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(1)
            .user_hauls(2)
            .hauls(2)
            .hauls(2)
            .user_hauls()
            .up()
            .tra(3)
            .landings(5)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(trip.hauls.len(), 6);
        assert_eq!(trip.tra.len(), 3);
        assert_eq!(trip.landing_ids.len(), 5);
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_are_not_added_to_trip_if_they_are_not_contained_within_its_period() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .user_hauls(1)
            .trips(1)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(trip.hauls.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_are_added_to_current_trip() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .dep(1)
            .user_hauls(2)
            .hauls(2)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);
        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(trip.hauls.len(), 4);
        assert!(trip.hauls[0].is_user_haul());
        assert!(trip.hauls[1].is_user_haul());
        assert!(!trip.hauls[2].is_user_haul());
        assert!(!trip.hauls[3].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_are_removed_from_current_trip_when_deleted() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .dep(1)
            .user_hauls(1)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        helper
            .app
            .delete_user_haul(state.user_hauls[0].id)
            .await
            .unwrap();

        helper.run_processors().await;

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert!(trip.hauls.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_user_hauls_are_removed_from_current_trip_when_moved_prior_to_current_trip_start() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .dep(1)
            .user_hauls(1)
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        let mut update = UpdateUserHaul::test_default();
        update.start_ts = state.dep[0].timestamp - Duration::seconds(10);
        update.end_ts = state.dep[0].timestamp - Duration::seconds(5);
        helper
            .app
            .update_user_haul(state.user_hauls[0].id, &update)
            .await
            .unwrap();

        helper.run_processors().await;

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert!(trip.hauls.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_cannot_delete_current_active_haul() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().build().await;

        helper.app.login_user_with_id(state.user_id);

        let started_haul = helper
            .app
            .start_user_haul(&HaulStart::test_default())
            .await
            .unwrap();

        let error = helper
            .app
            .delete_user_haul(started_haul.id)
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.error, ErrorDiscriminants::CannotModifyActiveUserHaul);
    })
    .await;
}

#[tokio::test]
async fn test_user_haul_is_not_connected_to_trips_if_it_overlaps_multiple() {
    test(|mut helper, builder| async move {
        let state = builder.vessel_with_test_call_sign().trips(2).build().await;

        helper.app.login_user_with_id(state.user_id);

        let start = HaulStart::test_default();
        let end = HaulEnd::test_default();

        helper.app.start_user_haul(&start).await.unwrap();
        let finished_haul = helper.app.stop_user_haul(&end).await.unwrap();

        let mut update = UpdateUserHaul::test_default();
        update.start_ts = state.trips[0].period.start();
        update.end_ts = state.trips[1].period.end();

        helper
            .app
            .update_user_haul(finished_haul.id, &update)
            .await
            .unwrap();

        helper.run_processors().await;
        helper.run_engine_cycle().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(trips[0].hauls.is_empty());
        assert!(trips[1].hauls.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_trips_dont_contain_user_hauls_when_user_is_not_logged_in() {
    test(|helper, builder| async move {
        builder
            .vessel_with_test_call_sign()
            .trips(1)
            .hauls(2)
            .user_hauls()
            .build()
            .await;

        helper.run_engine_cycle().await;

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert!(!trips.hauls[0].is_user_haul());
        assert!(!trips.hauls[1].is_user_haul());
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_dont_contain_user_hauls_when_user_is_not_logged_in() {
    test(|helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .dep(1)
            .hauls(2)
            .user_hauls(2)
            .build()
            .await;

        helper.run_engine_cycle().await;

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].id())
            .await
            .unwrap()
            .unwrap();

        assert!(!trip.hauls.iter().any(|t| t.is_user_haul()));
    })
    .await;
}

#[tokio::test]
async fn test_adding_new_ers_data_connects_to_user_hauls() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessel_with_test_call_sign()
            .trips(2)
            .user_hauls(2)
            .build()
            .await;

        helper
            .builder()
            .await
            .hauls(1)
            .modify(|h| {
                h.dca.vessel_info.id = Some(state.vessels[0].id());
                h.dca.set_start_timestamp(state.trips[0].period.start());
                h.dca.set_stop_timestamp(state.trips[0].period.end());
            })
            .build()
            .await;

        helper.app.login_user_with_id(state.user_id);

        let trips = helper
            .app
            .get_trips(TripsParameters {
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(trips[0].hauls[0].is_user_haul());
        assert!(!trips[1].hauls[0].is_user_haul());
    })
    .await;
}
