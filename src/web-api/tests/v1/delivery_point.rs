use std::array;

use super::helper::test;
use engine::*;
use fiskeridir_rs::DeliveryPointId;
use kyogre_core::*;

#[tokio::test]
async fn test_delivery_points_returns_aqua_culture_register() {
    test(|helper, builder| async move {
        let state = builder
            .delivery_points(3)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .landings(3)
            .build()
            .await;

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
        let state = builder
            .delivery_points(3)
            .with_delivery_point_source(DeliveryPointSourceId::Mattilsynet)
            .landings(3)
            .build()
            .await;

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
    test(|helper, builder| async move {
        let ids = helper.db.all_delivery_point_ids().await;

        builder
            .landings(ids.len())
            .modify_idx(|i, v| {
                v.landing.delivery_point.id = Some(ids[i].clone());
            })
            .build()
            .await;

        let dps = helper.app.get_delivery_points().await.unwrap();
        assert_eq!(dps.len(), 365);
    })
    .await;
}

#[tokio::test]
async fn test_delivery_points_returns_buyer_register_delivery_points() {
    test(|helper, builder| async move {
        let state = builder
            .delivery_points(3)
            .with_delivery_point_source(DeliveryPointSourceId::BuyerRegister)
            .landings(3)
            .build()
            .await;

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
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name("A");
            })
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::Mattilsynet)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name("B");
            })
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::Manual)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name("C");
            })
            .landings(1)
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
            .landings(1)
            .modify(|v| {
                v.landing.delivery_point.id = Some(id.clone());
            })
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name(old_name);
            })
            .persist()
            .await
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name(new_name);
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
            .landings(1)
            .modify(|v| {
                v.landing.delivery_point.id = Some(id.clone());
            })
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name(name);
            })
            .persist()
            .await
            .delivery_points(1)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .modify(|v| {
                v.val.set_id(id.clone());
                v.val.set_name(name);
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
        let ids = array::from_fn::<_, 3, _>(|i| DeliveryPointId::new_unchecked(format!("TEST{i}")));

        builder
            .delivery_points(3)
            .with_delivery_point_source(DeliveryPointSourceId::AquaCultureRegister)
            .modify_idx(|i, v| {
                v.val.set_id(ids[i].clone());
            })
            .build()
            .await;

        let [a, b, c] = ids;

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
        let old = DeliveryPointId::new_unchecked("B");

        builder
            .delivery_points(2)
            .modify_idx(|i, v| match i {
                0 => v.val.set_id(old.clone()),
                1 => v.val.set_id(new.clone()),
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

#[tokio::test]
async fn test_new_landing_increments_delivery_point_num_landings() {
    test(|helper, builder| async move {
        let id = DeliveryPointId::new_unchecked("A");

        builder
            .delivery_points(1)
            .modify(|v| {
                v.val.set_id(id.clone());
            })
            .build()
            .await;

        let num_landings = helper.db.delivery_point_num_landings(&id).await;

        assert_eq!(num_landings, 0);

        helper
            .builder()
            .await
            .landings(2)
            .modify(|v| {
                v.landing.delivery_point.id = Some(id.clone());
            })
            .build()
            .await;

        let num_landings = helper.db.delivery_point_num_landings(&id).await;

        assert_eq!(num_landings, 2);
    })
    .await;
}

#[tokio::test]
async fn test_deleting_landing_decrements_delivery_point_num_landings() {
    test(|helper, builder| async move {
        let state = builder.delivery_points(1).landings(2).build().await;

        let id = &state.delivery_points[0].id;

        let num_landings = helper.db.delivery_point_num_landings(id).await;

        assert_eq!(num_landings, 2);

        helper
            .builder()
            .await
            .landings(1)
            .modify(|v| {
                v.landing.delivery_point.id = Some(id.clone());
            })
            .build()
            .await;

        let num_landings = helper.db.delivery_point_num_landings(id).await;

        assert_eq!(num_landings, 1);
    })
    .await;
}

#[tokio::test]
async fn test_adding_delivery_point_sets_num_landings() {
    test(|helper, builder| async move {
        let id = DeliveryPointId::new_unchecked("A");

        builder
            .landings(2)
            .modify(|v| {
                v.landing.delivery_point.id = Some(id.clone());
            })
            .build()
            .await;

        let mut dp = MattilsynetDeliveryPoint::test_default();
        dp.id = id.clone();

        helper
            .db
            .db
            .add_mattilsynet_delivery_points(vec![dp])
            .await
            .unwrap();

        let num_landings = helper.db.delivery_point_num_landings(&id).await;
        assert_eq!(num_landings, 2);
    })
    .await;
}
