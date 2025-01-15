use super::helper::test;
use chrono::Duration;
use engine::*;

#[tokio::test]
async fn test_current_trip_returns_current_trip_without_prior_trip() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .fishing_facilities(2)
            .hauls(2)
            .build()
            .await;

        helper.app.login_user();
        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            state.dep[0].timestamp.timestamp_millis()
        );
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_current_trip_with_prior_trips() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .trips(1)
            .fishing_facilities(2)
            .hauls(2)
            .up()
            .dep(1)
            .fishing_facilities(2)
            .hauls(2)
            .build()
            .await;

        helper.app.login_user();
        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            state.dep[1].timestamp.timestamp_millis()
        );
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_null_when_no_current_trip() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).build().await;

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap();

        assert!(trip.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_does_not_include_fishing_facilities_without_token() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .fishing_facilities(2)
            .hauls(2)
            .build()
            .await;

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            state.dep[0].timestamp.timestamp_millis()
        );
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_does_not_include_fishing_facilities_without_permission() {
    test(|mut helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .fishing_facilities(2)
            .hauls(2)
            .build()
            .await;

        helper.app.login_user_with_policies(vec![]);

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            state.dep[0].timestamp.timestamp_millis()
        );
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_earliest_departure_since_previous_trip() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).up().dep(2).build().await;

        let trip = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            state.dep[1].timestamp.timestamp_millis()
        );
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_updates_correctly() {
    test(|helper, builder| async move {
        let state = builder.vessels(1).trips(1).up().dep(1).build().await;

        let vessel_id = state.vessels[0].fiskeridir.id;

        let trip = helper
            .app
            .get_current_trip(vessel_id)
            .await
            .unwrap()
            .unwrap();

        let prev_dep_timestamp = state.dep[1].timestamp;
        let new_dep_timestamp = prev_dep_timestamp + Duration::hours(10);

        assert_eq!(
            trip.departure.timestamp_millis(),
            prev_dep_timestamp.timestamp_millis(),
        );

        helper
            .builder()
            .await
            .vessels(1)
            .modify(|v| {
                v.fiskeridir.id = vessel_id;
            })
            .trips(1)
            .modify(|v| {
                v.trip_specification
                    .set_start(prev_dep_timestamp + Duration::hours(1));
                v.trip_specification
                    .set_end(prev_dep_timestamp + Duration::hours(2));
            })
            .up()
            .dep(1)
            .modify(|v| {
                v.dep.set_departure_timestamp(new_dep_timestamp);
            })
            .build()
            .await;

        let trip = helper
            .app
            .get_current_trip(vessel_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            new_dep_timestamp.timestamp_millis(),
        );
    })
    .await;
}
