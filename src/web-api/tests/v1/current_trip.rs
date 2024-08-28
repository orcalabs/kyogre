use super::helper::test;
use engine::*;
use reqwest::StatusCode;
use web_api::routes::v1::trip::CurrentTrip;

#[tokio::test]
async fn test_current_trip_returns_current_trip_without_prior_trip() {
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .fishing_facilities(2)
            .hauls(2)
            .build()
            .await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id, Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

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
    test(|helper, builder| async move {
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

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id, Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

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

        let response = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: Option<CurrentTrip> = response.json().await.unwrap();

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

        let response = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

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
    test(|helper, builder| async move {
        let state = builder
            .vessels(1)
            .dep(1)
            .fishing_facilities(2)
            .hauls(2)
            .build()
            .await;

        let token = helper.bw_helper.get_bw_token_with_policies(vec![]);
        let response = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id, Some(token))
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

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

        let response = helper
            .app
            .get_current_trip(state.vessels[0].fiskeridir.id, None)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(
            trip.departure.timestamp_millis(),
            state.dep[1].timestamp.timestamp_millis()
        );
    })
    .await;
}
