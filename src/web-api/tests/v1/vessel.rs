use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{Duration, TimeZone, Utc};
use fiskeridir_rs::{CallSign, GearGroup, Landing};
use kyogre_core::{FiskeridirVesselId, Mmsi, ScraperInboundPort, VesselBenchmarkId};
use web_api::routes::v1::vessel::Vessel;

#[tokio::test]
async fn test_vessels_returns_merged_data_from_fiskeridir_and_ais() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("LK-28").unwrap();
        let ais_vessel = helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;

        let vessel_id = 1;
        let mut landing = Landing::test_default(1, Some(vessel_id));
        landing.vessel.call_sign = Some(call_sign);

        helper
            .adapter()
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        let received_vessel = body.pop().unwrap();

        assert_eq!(received_vessel.fiskeridir, landing.vessel);
        assert_eq!(received_vessel.ais.unwrap(), ais_vessel);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_contains_weight_per_hour_benchmark() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(100, 0).unwrap();
        let end = start + Duration::hours(1);

        helper
            .db
            .generate_landing(1, vessel_id, start + Duration::seconds(1))
            .await;

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper.do_benchmarks().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = body.pop().unwrap();

        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            helper
                .db
                .benchmark(vessel.fiskeridir.id, VesselBenchmarkId::WeightPerHour)
                .await
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_is_correct_over_multiple_trips() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(100, 0).unwrap();
        let end = start + Duration::hours(1);

        let start2 = Utc.timestamp_opt(100000000, 0).unwrap();
        let end2 = start2 + Duration::hours(1);

        helper
            .db
            .generate_landing(1, vessel_id, start + Duration::seconds(1))
            .await;

        let trip = helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper
            .db
            .generate_landing(2, vessel_id, start2 + Duration::seconds(1))
            .await;

        let trip2 = helper.generate_ers_trip(vessel_id, &start2, &end2).await;

        helper.do_benchmarks().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = body.pop().unwrap();

        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            helper
                .db
                .benchmark(vessel.fiskeridir.id, VesselBenchmarkId::WeightPerHour)
                .await
        );
        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            (trip.delivery.total_living_weight + trip2.delivery.total_living_weight) / 2.0
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_includes_landings_not_covered_by_trips() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(100, 0).unwrap();
        let end = start + Duration::hours(1);

        helper
            .db
            .generate_landing(1, vessel_id, start + Duration::seconds(1))
            .await;

        let trip = helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper
            .db
            .generate_landing(2, vessel_id, end + Duration::seconds(1))
            .await;

        helper.do_benchmarks().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = body.pop().unwrap();

        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            helper
                .db
                .benchmark(vessel.fiskeridir.id, VesselBenchmarkId::WeightPerHour)
                .await
        );
        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            trip.delivery.total_living_weight * 2.0
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_excludes_landings_from_other_vessels() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        let vessel_id2 = FiskeridirVesselId(2);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id2, None, None)
            .await;

        let start = Utc.timestamp_opt(100, 0).unwrap();
        let end = start + Duration::hours(1);

        let start2 = Utc.timestamp_opt(100000000, 0).unwrap();
        let end2 = start2 + Duration::hours(1);

        helper
            .db
            .generate_landing(1, vessel_id, start + Duration::seconds(1))
            .await;

        let trip = helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper
            .db
            .generate_landing(2, vessel_id2, start2 + Duration::seconds(1))
            .await;

        helper.generate_ers_trip(vessel_id2, &start2, &end2).await;

        helper.do_benchmarks().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 2);
        let vessel = body.iter().find(|v| v.fiskeridir.id == vessel_id).unwrap();

        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            helper
                .db
                .benchmark(vessel.fiskeridir.id, VesselBenchmarkId::WeightPerHour)
                .await
        );
        assert_eq!(
            vessel.fish_caught_per_hour.unwrap(),
            trip.delivery.total_living_weight
        );
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_is_zero_if_there_are_trips_but_no_landings() {
    test(|mut helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let start = Utc.timestamp_opt(100, 0).unwrap();
        let end = start + Duration::hours(1);

        helper.generate_ers_trip(vessel_id, &start, &end).await;

        helper.do_benchmarks().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = &body[0];

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_weight_per_hour_is_zero_if_there_are_landings_but_no_trips() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        helper
            .db
            .generate_landing(1, vessel_id, Utc.timestamp_opt(100, 0).unwrap())
            .await;

        helper.do_benchmarks().await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);
        let vessel = &body[0];

        assert_eq!(vessel.fish_caught_per_hour.unwrap(), 0.0);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_zero_gear_groups_with_no_landings() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId(1);
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert!(vessel.gear_groups.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_vessel_has_gear_groups_of_landings() {
    test(|helper| async move {
        let vessel_id = 1;
        let mut landing = Landing::test_default(1, Some(vessel_id));
        landing.gear.group = GearGroup::Not;
        let mut landing2 = Landing::test_default(2, Some(vessel_id));
        landing2.gear.group = GearGroup::Garn;

        helper
            .adapter()
            .add_landings(vec![landing, landing2], 2023)
            .await
            .unwrap();

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![GearGroup::Not, GearGroup::Garn], vessel.gear_groups);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_removes_gear_group_when_last_landing_is_replaced_with_new_gear_group() {
    test(|helper| async move {
        let vessel_id = 1;
        let mut landing = Landing::test_default(1, Some(vessel_id));
        landing.gear.group = GearGroup::Not;

        let mut landing2 = landing.clone();
        landing2.document_info.version_number += 1;
        landing2.gear.group = GearGroup::Garn;

        helper
            .adapter()
            .add_landings(vec![landing], 2023)
            .await
            .unwrap();

        helper
            .adapter()
            .add_landings(vec![landing2], 2023)
            .await
            .unwrap();

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(body.len(), 1);

        let vessel = &body[0];
        assert_eq!(vec![GearGroup::Garn], vessel.gear_groups);
    })
    .await;
}
