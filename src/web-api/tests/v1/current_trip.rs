use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{TimeZone, Utc};
use fiskeridir_rs::{CallSign, ErsDep};
use kyogre_core::{FiskeridirVesselId, ScraperInboundPort};
use web_api::routes::v1::trip::CurrentTrip;

#[tokio::test]
async fn test_current_trip_returns_current_trip_without_prior_trip() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_current_trip(vessel_id, Some(token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_current_trip_with_prior_trips() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start1 = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();
        let end1 = Utc.timestamp_opt(400000, 1).unwrap();

        let start2 = Utc.timestamp_opt(500000, 1).unwrap();
        let middle3 = Utc.timestamp_opt(600000, 1).unwrap();
        let middle4 = Utc.timestamp_opt(700000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        helper.generate_ers_trip(vessel_id, &start1, &end1).await;

        let mut facility3 = kyogre_core::FishingFacility::test_default();
        let mut facility4 = kyogre_core::FishingFacility::test_default();
        facility3.call_sign = Some(call_sign.clone());
        facility3.setup_timestamp = middle3;
        facility3.removed_timestamp = Some(middle4);
        facility4.call_sign = Some(call_sign.clone());
        facility4.setup_timestamp = middle3;
        facility4.removed_timestamp = Some(middle4);

        helper
            .db
            .add_fishing_facilities(vec![facility3, facility4])
            .await;

        helper.db.generate_haul(vessel_id, &middle3, &middle3).await;
        helper.db.generate_haul(vessel_id, &middle4, &middle4).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start2, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_current_trip(vessel_id, Some(token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start2.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_null_when_no_current_trip() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let end = Utc.timestamp_opt(200000, 1).unwrap();

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        let response = helper.app.get_current_trip(vessel_id, None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: Option<CurrentTrip> = response.json().await.unwrap();

        assert!(trip.is_none());
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_does_not_include_fishing_facilities_without_token() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let response = helper.app.get_current_trip(vessel_id, None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_does_not_include_fishing_facilities_without_permission() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start = Utc.timestamp_opt(100000, 1).unwrap();
        let middle1 = Utc.timestamp_opt(200000, 1).unwrap();
        let middle2 = Utc.timestamp_opt(300000, 1).unwrap();

        let mut facility1 = kyogre_core::FishingFacility::test_default();
        let mut facility2 = kyogre_core::FishingFacility::test_default();
        facility1.call_sign = Some(call_sign.clone());
        facility1.setup_timestamp = middle1;
        facility1.removed_timestamp = Some(middle2);
        facility2.call_sign = Some(call_sign.clone());
        facility2.setup_timestamp = middle1;
        facility2.removed_timestamp = Some(middle2);

        helper
            .db
            .add_fishing_facilities(vec![facility1, facility2])
            .await;

        helper.db.generate_haul(vessel_id, &middle1, &middle1).await;
        helper.db.generate_haul(vessel_id, &middle2, &middle2).await;

        let departure = ErsDep::test_default(1, vessel_id.0 as u64, start, 1);
        helper.db.db.add_ers_dep(vec![departure]).await.unwrap();

        let token = helper.bw_helper.get_bw_token_with_policies(vec![]);
        let response = helper.app.get_current_trip(vessel_id, Some(token)).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start.timestamp_millis());
        assert_eq!(trip.target_species_fiskeridir_id, Some(1021));
        assert_eq!(trip.hauls.len(), 2);
        assert_eq!(trip.fishing_facilities.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_current_trip_returns_earliest_departure_since_previous_trip() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(10);
        let call_sign = CallSign::new_unchecked("LK17");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, Some(call_sign.clone()))
            .await;

        let start1 = Utc.timestamp_opt(100000, 1).unwrap();
        let end1 = Utc.timestamp_opt(200000, 1).unwrap();
        let start2 = Utc.timestamp_opt(300000, 1).unwrap();
        let start3 = Utc.timestamp_opt(400000, 1).unwrap();

        helper.generate_ers_trip(vessel_id, &start1, &end1).await;

        let departure1 = ErsDep::test_default(1, vessel_id.0 as u64, start3, 1);
        let departure2 = ErsDep::test_default(2, vessel_id.0 as u64, start2, 2);
        helper
            .db
            .db
            .add_ers_dep(vec![departure2, departure1])
            .await
            .unwrap();

        let response = helper.app.get_current_trip(vessel_id, None).await;
        assert_eq!(response.status(), StatusCode::OK);

        let trip: CurrentTrip = response.json().await.unwrap();

        assert_eq!(trip.departure.timestamp_millis(), start2.timestamp_millis());
    })
    .await;
}
