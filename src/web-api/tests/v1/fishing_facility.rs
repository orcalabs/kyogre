use super::helper::test;
use actix_web::http::StatusCode;
use web_api::routes::v1::fishing_facility::FishingFacility;

#[tokio::test]
async fn test_fishing_facility_historic_returns_all_historic_fishing_facilities() {
    test(|helper| async move {
        let mut expected = vec![
            helper.db.generate_fishing_facility_historic().await,
            helper.db.generate_fishing_facility_historic().await,
            helper.db.generate_fishing_facility_historic().await,
        ];

        let response = helper.app.get_fishing_facility_historic().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut facilities: Vec<FishingFacility> = response.json().await.unwrap();

        expected.sort_by_key(|f| f.tool_id.as_u128());
        facilities.sort_by_key(|f| f.tool_id.as_u128());

        assert_eq!(facilities.len(), 3);
        assert_eq!(facilities, expected);
    })
    .await;
}
