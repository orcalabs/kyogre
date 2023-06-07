use super::helper::test;
use actix_web::http::StatusCode;
use fiskeridir_rs::CallSign;
use kyogre_core::{
    FishingFacilitiesSorting, FishingFacilityToolType, FiskeridirVesselId, Mmsi, Ordering,
};
use web_api::routes::v1::fishing_facility::{FishingFacilitiesParams, FishingFacility};

#[tokio::test]
async fn test_fishing_facilities_returns_all_fishing_facilities() {
    test(|helper| async move {
        let mut expected = vec![
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
        ];

        let token = helper.bw_helper.get_bw_token();
        let response = helper
            .app
            .get_fishing_facilities(Default::default(), token)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_mmsis() {
    test(|helper| async move {
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;

        let mut expected = vec![
            kyogre_core::FishingFacility::test_default(),
            kyogre_core::FishingFacility::test_default(),
        ];

        let mmsi1 = Mmsi(42);
        let mmsi2 = Mmsi(43);

        expected[0].mmsi = Some(mmsi1);
        expected[1].mmsi = Some(mmsi2);

        helper.db.add_fishing_facilities(expected.clone()).await;

        let params = FishingFacilitiesParams {
            mmsis: Some(vec![mmsi1, mmsi2]),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_vessel_ids() {
    test(|helper| async move {
        let vessel_id1 = FiskeridirVesselId(10);
        let vessel_id2 = FiskeridirVesselId(20);
        let call_sign1 = CallSign::new_unchecked("AAAAAA");
        let call_sign2 = CallSign::new_unchecked("BBBBBB");
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id1, None, Some(call_sign1.clone()))
            .await;
        helper
            .db
            .generate_fiskeridir_vessel(vessel_id2, None, Some(call_sign2.clone()))
            .await;

        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;

        let mut expected = vec![
            kyogre_core::FishingFacility::test_default(),
            kyogre_core::FishingFacility::test_default(),
        ];

        expected[0].call_sign = Some(call_sign1);
        expected[1].call_sign = Some(call_sign2);

        helper.db.add_fishing_facilities(expected.clone()).await;

        let params = FishingFacilitiesParams {
            fiskeridir_vessel_ids: Some(vec![vessel_id1, vessel_id2]),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_with_tool_types() {
    test(|helper| async move {
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;

        let mut expected = vec![
            kyogre_core::FishingFacility::test_default(),
            kyogre_core::FishingFacility::test_default(),
        ];

        expected[0].tool_type = FishingFacilityToolType::Sensorbuoy;
        expected[1].tool_type = FishingFacilityToolType::Crabpot;

        helper.db.add_fishing_facilities(expected.clone()).await;

        let params = FishingFacilitiesParams {
            tool_types: Some(vec![
                FishingFacilityToolType::Sensorbuoy,
                FishingFacilityToolType::Crabpot,
            ]),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_active_fishing_facilities() {
    test(|helper| async move {
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;

        let mut expected = vec![
            kyogre_core::FishingFacility::test_default(),
            kyogre_core::FishingFacility::test_default(),
        ];

        expected[0].removed_timestamp = None;
        expected[1].removed_timestamp = None;

        helper.db.add_fishing_facilities(expected.clone()).await;

        let params = FishingFacilitiesParams {
            active: Some(true),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_inactive_fishing_facilities() {
    test(|helper| async move {
        let mut expected = vec![
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
        ];

        let mut f1 = kyogre_core::FishingFacility::test_default();
        let mut f2 = kyogre_core::FishingFacility::test_default();
        let mut f3 = kyogre_core::FishingFacility::test_default();

        f1.removed_timestamp = None;
        f2.removed_timestamp = None;
        f3.removed_timestamp = None;

        helper.db.add_fishing_facilities(vec![f1, f2, f3]).await;

        let params = FishingFacilitiesParams {
            active: Some(false),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_in_setup_ranges() {
    test(|helper| async move {
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;

        let mut expected = vec![
            kyogre_core::FishingFacility::test_default(),
            kyogre_core::FishingFacility::test_default(),
        ];

        expected[0].setup_timestamp = "2000-01-10T00:00:00Z".parse().unwrap();
        expected[1].setup_timestamp = "2000-01-20T00:00:00Z".parse().unwrap();

        helper.db.add_fishing_facilities(expected.clone()).await;

        let params = FishingFacilitiesParams {
            setup_ranges: Some(vec![
                "[2000-01-05T00:00:00Z,2000-01-15T00:00:00Z]"
                    .parse()
                    .unwrap(),
                "[2000-01-15T00:00:00Z,2000-01-25T00:00:00Z]"
                    .parse()
                    .unwrap(),
            ]),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_returns_fishing_facilities_in_removed_ranges() {
    test(|helper| async move {
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;
        helper.db.generate_fishing_facility().await;

        let mut expected = vec![
            kyogre_core::FishingFacility::test_default(),
            kyogre_core::FishingFacility::test_default(),
        ];

        expected[0].removed_timestamp = Some("2000-01-10T00:00:00Z".parse().unwrap());
        expected[1].removed_timestamp = Some("2000-01-20T00:00:00Z".parse().unwrap());

        helper.db.add_fishing_facilities(expected.clone()).await;

        let params = FishingFacilitiesParams {
            removed_ranges: Some(vec![
                "[2000-01-05T00:00:00Z,2000-01-15T00:00:00Z]"
                    .parse()
                    .unwrap(),
                "[2000-01-15T00:00:00Z,2000-01-25T00:00:00Z]"
                    .parse()
                    .unwrap(),
            ]),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 2);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_fails_without_bw_token() {
    test(|helper| async move {
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
    test(|helper| async move {
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
    test(|helper| async move {
        let mut expected = vec![
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
        ];

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

        expected.sort_by_key(|f| f.setup_timestamp);
        expected = expected.into_iter().skip(2).take(3).collect();

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_sorts_by_removed_timestamp() {
    test(|helper| async move {
        let mut expected = vec![
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
        ];

        let params = FishingFacilitiesParams {
            ordering: Some(Ordering::Asc),
            sorting: Some(FishingFacilitiesSorting::Removed),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.removed_timestamp);

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, expected);
    })
    .await;
}

#[tokio::test]
async fn test_fishing_facilities_sorts_by_last_changed() {
    test(|helper| async move {
        let mut expected = vec![
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
            helper.db.generate_fishing_facility().await,
        ];

        let params = FishingFacilitiesParams {
            ordering: Some(Ordering::Asc),
            sorting: Some(FishingFacilitiesSorting::LastChanged),
            ..Default::default()
        };

        let token = helper.bw_helper.get_bw_token();
        let response = helper.app.get_fishing_facilities(params, token).await;

        assert_eq!(response.status(), StatusCode::OK);
        let facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.last_changed);

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, expected);
    })
    .await;
}
