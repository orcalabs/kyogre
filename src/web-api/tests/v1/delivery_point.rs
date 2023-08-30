use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{TimeZone, Utc};
use fiskeridir_rs::{AquaCultureEntry, DeliveryPointId};
use kyogre_core::{FiskeridirVesselId, MattilsynetDeliveryPoint, ScraperInboundPort};
use web_api::routes::v1::{delivery_point::DeliveryPoint, landing::Landing};

#[tokio::test]
async fn test_delivery_points_returns_aqua_culture_register() {
    test(|helper, _builder| async move {
        let mut entries = vec![
            AquaCultureEntry::test_default(),
            AquaCultureEntry::test_default(),
            AquaCultureEntry::test_default(),
        ];

        entries[0].delivery_point_id = DeliveryPointId::new_unchecked("A");
        entries[1].delivery_point_id = DeliveryPointId::new_unchecked("B");
        entries[2].delivery_point_id = DeliveryPointId::new_unchecked("C");

        let ids = vec![
            entries[0].delivery_point_id.clone(),
            entries[1].delivery_point_id.clone(),
            entries[2].delivery_point_id.clone(),
        ];

        helper
            .db
            .db
            .add_aqua_culture_register(entries.clone())
            .await
            .unwrap();

        let response = helper.app.get_delivery_points().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut dps: Vec<DeliveryPoint> = response.json().await.unwrap();

        dps.retain(|v| ids.contains(&v.id));
        dps.sort_by_key(|d| d.id.clone());

        assert_eq!(dps.len(), 3);
        assert_eq!(dps, entries);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_returns_mattilsynet_delivery_points() {
    test(|helper, _builder| async move {
        let mut delivery_points = vec![
            MattilsynetDeliveryPoint::test_default(),
            MattilsynetDeliveryPoint::test_default(),
            MattilsynetDeliveryPoint::test_default(),
        ];

        delivery_points[0].id = DeliveryPointId::new_unchecked("A");
        delivery_points[1].id = DeliveryPointId::new_unchecked("B");
        delivery_points[2].id = DeliveryPointId::new_unchecked("C");

        let ids = vec![
            delivery_points[0].id.clone(),
            delivery_points[1].id.clone(),
            delivery_points[2].id.clone(),
        ];

        helper
            .db
            .db
            .add_mattilsynet_delivery_points(delivery_points.clone())
            .await
            .unwrap();

        let response = helper.app.get_delivery_points().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut dps: Vec<DeliveryPoint> = response.json().await.unwrap();

        dps.retain(|v| ids.contains(&v.id));
        dps.sort_by_key(|d| d.id.clone());
        let core = delivery_points
            .into_iter()
            .map(kyogre_core::DeliveryPoint::from)
            .collect::<Vec<_>>();

        assert_eq!(dps.len(), 3);
        assert_eq!(dps, core);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_returns_manual_delivery_points() {
    test(|helper, _builder| async move {
        let response = helper.app.get_delivery_points().await;

        assert_eq!(response.status(), StatusCode::OK);
        let dps: Vec<DeliveryPoint> = response.json().await.unwrap();

        assert_eq!(dps.len(), 331);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_prioritizes_manual_entries() {
    test(|helper, _builder| async move {
        let id = DeliveryPointId::new_unchecked("A");

        let mut entry = AquaCultureEntry::test_default();
        let mut dp = MattilsynetDeliveryPoint::test_default();

        entry.delivery_point_id = id.clone();
        entry.name = "A".into();

        dp.id = id.clone();
        dp.name = "B".into();

        helper
            .db
            .db
            .add_aqua_culture_register(vec![entry])
            .await
            .unwrap();

        helper
            .db
            .db
            .add_mattilsynet_delivery_points(vec![dp])
            .await
            .unwrap();

        helper
            .db
            .add_manual_delivery_point(id.clone(), "C".into())
            .await;

        let response = helper.app.get_delivery_points().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut dps: Vec<DeliveryPoint> = response.json().await.unwrap();
        dps.retain(|v| v.id == id);

        assert_eq!(dps.len(), 1);
        assert_eq!(dps[0].id, id);
        assert_eq!(dps[0].name, Some("C".into()));
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_adds_to_log_when_updated() {
    test(|helper, _builder| async move {
        let mut entry = AquaCultureEntry::test_default();

        helper
            .db
            .db
            .add_aqua_culture_register(vec![entry.clone()])
            .await
            .unwrap();

        let old_name = entry.name.clone();
        entry.name = "New name".into();

        helper
            .db
            .db
            .add_aqua_culture_register(vec![entry.clone()])
            .await
            .unwrap();

        let response = helper.app.get_delivery_points().await;

        assert_eq!(response.status(), StatusCode::OK);

        let mut dps: Vec<DeliveryPoint> = response.json().await.unwrap();
        dps.retain(|v| v.id == entry.delivery_point_id);

        assert_eq!(dps.len(), 1);
        assert_eq!(dps[0].id, entry.delivery_point_id);
        assert_eq!(dps[0].name, Some(entry.name.clone()));

        let log = helper.db.get_delivery_points_log().await;

        assert_eq!(log.len(), 1);
        assert_eq!(
            log[0].get("delivery_point_id").unwrap().as_str().unwrap(),
            entry.delivery_point_id.into_inner()
        );
        assert_eq!(
            log[0]
                .get("old_value")
                .unwrap()
                .get("name")
                .unwrap()
                .as_str()
                .unwrap(),
            old_name
        );
        assert_eq!(
            log[0]
                .get("new_value")
                .unwrap()
                .get("name")
                .unwrap()
                .as_str()
                .unwrap(),
            entry.name
        );
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_doesnt_add_to_log_when_updated_without_change() {
    test(|helper, _builder| async move {
        let entry = vec![AquaCultureEntry::test_default()];

        helper
            .db
            .db
            .add_aqua_culture_register(entry.clone())
            .await
            .unwrap();

        helper
            .db
            .db
            .add_aqua_culture_register(entry.clone())
            .await
            .unwrap();

        let response = helper.app.get_delivery_points().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut dps: Vec<DeliveryPoint> = response.json().await.unwrap();
        dps.retain(|v| v.id == entry[0].delivery_point_id);

        assert_eq!(dps.len(), 1);

        let log = helper.db.get_delivery_points_log().await;

        assert_eq!(log.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_cant_add_deprecation_chain() {
    test(|helper, _builder| async move {
        let a = DeliveryPointId::new_unchecked("A");
        let b = DeliveryPointId::new_unchecked("B");
        let c = DeliveryPointId::new_unchecked("C");

        helper
            .db
            .add_deprecated_delivery_point(a, b.clone())
            .await
            .unwrap();
        let res = helper.db.add_deprecated_delivery_point(b, c).await;

        assert!(res.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_landings_respect_delivery_point_deprecation() {
    test(|helper, _builder| async move {
        let vessel_id = FiskeridirVesselId(111);
        let date = Utc.timestamp_opt(1000, 0).unwrap();

        let landing = helper.db.generate_landing(1, vessel_id, date).await;

        let id = DeliveryPointId::new_unchecked("A");

        helper
            .db
            .add_deprecated_delivery_point(landing.delivery_point_id.unwrap(), id.clone())
            .await
            .unwrap();

        let response = helper.app.get_landings(Default::default()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let landings: Vec<Landing> = response.json().await.unwrap();

        assert_eq!(landings.len(), 1);
        assert_eq!(landings[0].delivery_point_id, Some(id));
    })
    .await;
}
