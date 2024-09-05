use super::helper::test;
use engine::*;
use kyogre_core::{FishingFacilitiesSorting, FishingFacilityToolType, Mmsi, Ordering};
use reqwest::StatusCode;
use web_api::routes::v1::fishing_facility::FishingFacilitiesParams;

#[tokio::test]
async fn test_fishing_facilities_returns_all_fishing_facilities() {
    test(|mut helper, builder| async move {
        let state = builder.fishing_facilities(3).build().await;

        helper.app.login_user();

        let facilities = helper
            .app
            .get_fishing_facilities(FishingFacilitiesParams {
                ordering: Some(Ordering::Asc),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_mmsis() {
    test(|mut helper, builder| async move {
        let mmsi1 = Mmsi::test_new(42);
        let mmsi2 = Mmsi::test_new(43);

        let mut state = builder
            .fishing_facilities(2)
            .modify_idx(|i, v| match i {
                0 => v.facility.mmsi = Some(mmsi1),
                1 => v.facility.mmsi = Some(mmsi2),
                _ => unreachable!(),
            })
            .build()
            .await;

        let params = FishingFacilitiesParams {
            mmsis: Some(vec![mmsi1, mmsi2]),
            ..Default::default()
        };

        helper.app.login_user();
        let mut facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        state
            .fishing_facilities
            .sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_vessel_ids() {
    test(|mut helper, builder| async move {
        let state = builder.vessels(2).fishing_facilities(2).build().await;

        let params = FishingFacilitiesParams {
            fiskeridir_vessel_ids: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_tool_types() {
    test(|mut helper, builder| async move {
        let state = builder
            .fishing_facilities(5)
            .modify_idx(|i, v| match i {
                0 => v.facility.tool_type = FishingFacilityToolType::Sensorbuoy,
                1 => v.facility.tool_type = FishingFacilityToolType::Crabpot,
                _ => (),
            })
            .build()
            .await;

        let params = FishingFacilitiesParams {
            tool_types: Some(vec![
                FishingFacilityToolType::Sensorbuoy,
                FishingFacilityToolType::Crabpot,
            ]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_active_fishing_facilities() {
    test(|mut helper, builder| async move {
        let state = builder
            .fishing_facilities(5)
            .modify_idx(|i, v| {
                if let 0..=1 = i {
                    v.facility.removed_timestamp = None
                }
            })
            .build()
            .await;

        let params = FishingFacilitiesParams {
            active: Some(true),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_inactive_fishing_facilities() {
    test(|mut helper, builder| async move {
        let state = builder
            .fishing_facilities(5)
            .modify_idx(|i, v| {
                if let 0..=2 = i {
                    v.facility.removed_timestamp = None
                }
            })
            .build()
            .await;

        let params = FishingFacilitiesParams {
            active: Some(false),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[3..]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_in_setup_ranges() {
    test(|mut helper, builder| async move {
        let setup = "2000-01-10T00:00:00Z".parse().unwrap();
        let setup2 = "2000-01-20T00:00:00Z".parse().unwrap();

        let state = builder
            .fishing_facilities(5)
            .modify_idx(|i, v| match i {
                0 => v.facility.setup_timestamp = setup,
                1 => v.facility.setup_timestamp = setup2,
                _ => (),
            })
            .build()
            .await;

        let params = FishingFacilitiesParams {
            setup_ranges: Some(vec![
                "[2000-01-05T00:00:00Z,2000-01-15T00:00:00Z]"
                    .parse()
                    .unwrap(),
                "[2000-01-15T00:00:00Z,2000-01-25T00:00:00Z]"
                    .parse()
                    .unwrap(),
            ]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_in_removed_ranges() {
    test(|mut helper, builder| async move {
        let removed = "2000-01-10T00:00:00Z".parse().unwrap();
        let removed2 = "2000-01-20T00:00:00Z".parse().unwrap();

        let state = builder
            .fishing_facilities(5)
            .modify_idx(|i, v| match i {
                0 => v.facility.removed_timestamp = Some(removed),
                1 => v.facility.removed_timestamp = Some(removed2),
                _ => (),
            })
            .build()
            .await;

        let params = FishingFacilitiesParams {
            removed_ranges: Some(vec![
                "[2000-01-05T00:00:00Z,2000-01-15T00:00:00Z]"
                    .parse()
                    .unwrap(),
                "[2000-01-15T00:00:00Z,2000-01-25T00:00:00Z]"
                    .parse()
                    .unwrap(),
            ]),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_fails_without_bw_token() {
    test(|helper, _builder| async move {
        let error = helper
            .app
            .get_fishing_facilities(Default::default())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_fails_without_bw_read_extended_fishing_facility() {
    test(|mut helper, _builder| async move {
        helper.app.login_user_with_policies(vec![]);
        let error = helper
            .app
            .get_fishing_facilities(Default::default())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::FORBIDDEN);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_filters_by_limit_and_offset() {
    test(|mut helper, builder| async move {
        let state = builder.fishing_facilities(8).build().await;

        let params = FishingFacilitiesParams {
            offset: Some(2),
            limit: Some(3),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities[2..5]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_sorts_by_removed_timestamp() {
    test(|mut helper, builder| async move {
        let mut state = builder.fishing_facilities(3).build().await;

        let params = FishingFacilitiesParams {
            ordering: Some(Ordering::Asc),
            sorting: Some(FishingFacilitiesSorting::Removed),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        state
            .fishing_facilities
            .sort_by_key(|f| f.removed_timestamp);

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_sorts_by_last_changed() {
    test(|mut helper, builder| async move {
        let mut state = builder.fishing_facilities(3).build().await;
        let params = FishingFacilitiesParams {
            ordering: Some(Ordering::Asc),
            sorting: Some(FishingFacilitiesSorting::LastChanged),
            ..Default::default()
        };

        helper.app.login_user();
        let facilities = helper.app.get_fishing_facilities(params).await.unwrap();

        state.fishing_facilities.sort_by_key(|f| f.last_changed);

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}
