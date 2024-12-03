use super::helper::test;
use engine::*;
use fiskeridir_rs::DeliveryPointId;
use kyogre_core::*;

#[tokio::test]
async fn test_delivery_points_returns_aqua_culture_register() {
    test(|helper, builder| async move {
        let state = builder.aqua_cultures(3).build().await;

        let mut dps = helper.app.get_delivery_points().await.unwrap();
        dps.retain(|v| state.delivery_points.iter().any(|d| d.id == v.id));
        dps.sort_by_key(|d| d.id.clone());

        assert_eq!(dps.len(), 3);
        assert_eq!(dps, state.delivery_points);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_returns_mattilsynet_delivery_points() {
    test(|helper, builder| async move {
        let state = builder.mattilsynet(3).build().await;

        let mut dps = helper.app.get_delivery_points().await.unwrap();
        dps.retain(|v| state.delivery_points.iter().any(|d| d.id == v.id));
        dps.sort_by_key(|d| d.id.clone());

        assert_eq!(dps.len(), 3);
        assert_eq!(dps, state.delivery_points);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_returns_manual_delivery_points() {
    test(|helper, _builder| async move {
        let dps = helper.app.get_delivery_points().await.unwrap();
        assert_eq!(dps.len(), 365);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_returns_buyer_register_delivery_points() {
    test(|helper, builder| async move {
        let state = builder.buyer_locations(3).build().await;

        let mut dps = helper.app.get_delivery_points().await.unwrap();
        dps.retain(|v| state.delivery_points.iter().any(|d| d.id == v.id));
        dps.sort_by_key(|d| d.id.clone());

        assert_eq!(dps.len(), 3);
        assert_eq!(dps, state.delivery_points);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_prioritizes_manual_entries() {
    test(|helper, builder| async move {
        let id = DeliveryPointId::new_unchecked("A");

        let state = builder
            .aqua_cultures(1)
            .modify(|v| {
                v.val.delivery_point_id = id.clone();
                v.val.name = "A".parse().unwrap()
            })
            .mattilsynet(1)
            .modify(|v| {
                v.val.id = id.clone();
                v.val.name = "B".parse().unwrap()
            })
            .manual_delivery_points(1)
            .modify(|v| {
                v.val.id = id.clone();
                v.val.name = "C".parse().unwrap()
            })
            .build()
            .await;

        let mut dps = helper.app.get_delivery_points().await.unwrap();
        dps.retain(|v| v.id == id);

        assert_eq!(dps.len(), 1);
        assert_eq!(dps[0].id, id);
        assert_eq!(dps[0].name.as_deref(), Some("C"));
        assert_eq!(
            dps[0],
            *state.delivery_points.iter().find(|v| v.id == id).unwrap()
        );
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_adds_to_log_when_updated() {
    test(|helper, builder| async move {
        let old_name = "old_name";
        let new_name = "new_name";
        let id = DeliveryPointId::new_unchecked("A");

        let state = builder
            .aqua_cultures(1)
            .modify(|v| {
                v.val.delivery_point_id = id.clone();
                v.val.name = old_name.parse().unwrap();
            })
            .persist()
            .await
            .aqua_cultures(1)
            .modify(|v| {
                v.val.delivery_point_id = id.clone();
                v.val.name = new_name.parse().unwrap();
            })
            .build()
            .await;

        let mut dps = helper.app.get_delivery_points().await.unwrap();
        dps.retain(|v| v.id == state.delivery_points[0].id);

        assert_eq!(dps.len(), 1);
        let entry = &dps[0];
        assert_eq!(dps[0], state.delivery_points[0]);

        let log = helper.db.db.delivery_points_log().await;

        assert_eq!(log.len(), 1);
        assert_eq!(
            log[0].get("delivery_point_id").unwrap().as_str().unwrap(),
            entry.id.as_ref()
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
            entry.name.clone().unwrap()
        );
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_doesnt_add_to_log_when_updated_without_change() {
    test(|helper, builder| async move {
        let name = "name";
        let id = DeliveryPointId::new_unchecked("A");

        let state = builder
            .aqua_cultures(1)
            .modify(|v| {
                v.val.delivery_point_id = id.clone();
                v.val.name = name.parse().unwrap();
            })
            .persist()
            .await
            .aqua_cultures(1)
            .modify(|v| {
                v.val.delivery_point_id = id.clone();
                v.val.name = name.parse().unwrap();
            })
            .build()
            .await;

        let mut dps = helper.app.get_delivery_points().await.unwrap();
        dps.retain(|v| v.id == state.delivery_points[0].id);

        assert_eq!(dps.len(), 1);

        let log = helper.db.db.delivery_points_log().await;
        assert_eq!(log.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_cant_add_deprecation_chain() {
    test(|helper, builder| async move {
        let a = DeliveryPointId::new_unchecked("A");
        let b = DeliveryPointId::new_unchecked("B");
        let c = DeliveryPointId::new_unchecked("C");

        builder
            .aqua_cultures(3)
            .modify_idx(|i, v| match i {
                0 => v.val.delivery_point_id = a.clone(),
                1 => v.val.delivery_point_id = b.clone(),
                2 => v.val.delivery_point_id = c.clone(),
                _ => unreachable!(),
            })
            .build()
            .await;

        helper
            .db
            .db
            .add_deprecated_delivery_point(a, b.clone())
            .await
            .unwrap();
        let res = helper.db.db.add_deprecated_delivery_point(b, c).await;

        assert!(res.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_landings_respect_delivery_point_deprecation() {
    test(|helper, builder| async move {
        let new = DeliveryPointId::new_unchecked("A");
        let old = DeliveryPointId::new_unchecked("A");

        builder
            .aqua_cultures(2)
            .modify_idx(|i, v| match i {
                0 => v.val.delivery_point_id = old.clone(),
                1 => v.val.delivery_point_id = new.clone(),
                _ => unreachable!(),
            })
            .vessels(1)
            .landings(1)
            .modify(|l| {
                l.landing.delivery_point.id = Some(old.clone());
            })
            .build()
            .await;

        helper
            .db
            .db
            .add_deprecated_delivery_point(old, new.clone())
            .await
            .unwrap();

        let landings = helper.app.get_landings(Default::default()).await.unwrap();

        assert_eq!(landings.len(), 1);
        assert_eq!(landings[0].delivery_point_id, Some(new));
    })
    .await;
}
