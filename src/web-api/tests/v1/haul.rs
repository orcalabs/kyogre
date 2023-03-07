use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{DateTime, Utc};
use fiskeridir_rs::ErsDca;
use kyogre_core::ScraperInboundPort;
use web_api::routes::{
    utils::DateTimeUtc,
    v1::haul::{Haul, HaulsParams},
};

#[tokio::test]
async fn test_hauls_returns_all_hauls() {
    test(|helper| async move {
        helper.db.generate_ers_dca(1, None).await;
        helper.db.generate_ers_dca(2, None).await;
        helper.db.generate_ers_dca(3, None).await;

        let response = helper.app.get_hauls(Default::default()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_in_specified_months() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2001-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2000-06-1T00:00:00Z".parse().unwrap();

        ers1.start_timestamp = Some(month1);
        ers1.stop_timestamp = Some(month1);
        ers2.start_timestamp = Some(month2);
        ers2.stop_timestamp = Some(month2);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            months: Some(vec![DateTimeUtc(month1), DateTimeUtc(month2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_returns_hauls_in_catch_location() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();
        helper.db.db.update_database_views().await.unwrap();

        let params = HaulsParams {
            catch_locations: Some(vec![
                "09-05".try_into().unwrap(),
                "09-04".try_into().unwrap(),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls(params).await;

        assert_eq!(response.status(), StatusCode::OK);
        let hauls: Vec<Haul> = response.json().await.unwrap();

        assert_eq!(hauls.len(), 2);
    })
    .await;
}
