use super::helper::test;
use actix_web::http::StatusCode;
use kyogre_core::{FishingFacilitiesSorting, FishingFacilityToolType, Mmsi, Ordering};
use web_api::routes::v1::fishing_facility::{FishingFacilitiesParams, FishingFacility};

#[tokio::test]
async fn test_fishing_facilities_returns_all_fishing_facilities() {
    test(|helper, builder| async move {
        let state = builder.fishing_facilities(3).build().await;

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_fishing_facilities(
                FishingFacilitiesParams {
                    ordering: Some(Ordering::Asc),
                    ..Default::default()
                },
                token,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_mmsis() {
    test(|helper, builder| async move {
        let mmsi1 = Mmsi(42);
        let mmsi2 = Mmsi(43);

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

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

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
    test(|helper, builder| async move {
        let state = builder.vessels(2).fishing_facilities(2).build().await;

        let params = FishingFacilitiesParams {
            fiskeridir_vessel_ids: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_tool_types() {
    test(|helper, builder| async move {
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

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_active_fishing_facilities() {
    test(|helper, builder| async move {
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

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_inactive_fishing_facilities() {
    test(|helper, builder| async move {
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

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[3..]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_in_setup_ranges() {
    test(|helper, builder| async move {
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

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_in_removed_ranges() {
    test(|helper, builder| async move {
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

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, state.fishing_facilities[0..2]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_fails_without_bw_token() {
    test(|helper, _builder| async move {
        let response = helper
            .app
            .get_fishing_facilities(Default::default(), "".into())
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_fails_without_bw_read_extended_fishing_facility() {
    test(|helper, _builder| async move {
        let token = helper.bw_helper.get_bw_token_with_policies(vec![]);
        let response = helper
            .app
            .get_fishing_facilities(Default::default(), token)
            .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_filters_by_limit_and_offset() {
    test(|helper, builder| async move {
        let state = builder.fishing_facilities(8).build().await;

        let params = FishingFacilitiesParams {
            offset: Some(2),
            limit: Some(3),
            ordering: Some(Ordering::Asc),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities[2..5]);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_sorts_by_removed_timestamp() {
    test(|helper, builder| async move {
        let mut state = builder.fishing_facilities(3).build().await;

        let params = FishingFacilitiesParams {
            ordering: Some(Ordering::Asc),
            sorting: Some(FishingFacilitiesSorting::Removed),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

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
    test(|helper, builder| async move {
        let mut state = builder.fishing_facilities(3).build().await;
        let params = FishingFacilitiesParams {
            ordering: Some(Ordering::Asc),
            sorting: Some(FishingFacilitiesSorting::LastChanged),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        state.fishing_facilities.sort_by_key(|f| f.last_changed);

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, state.fishing_facilities);
    })
    .await;
}
