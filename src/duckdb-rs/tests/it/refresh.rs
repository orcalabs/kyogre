use chrono::{TimeZone, Utc};
use fiskeridir_rs::{ErsDca, Landing};
use kyogre_core::{
    ActiveHaulsFilter, ActiveLandingFilter, FiskeridirVesselId, HaulsMatrixQuery,
    LandingMatrixQuery, MatrixCacheOutbound,
};

use super::helper::test;

#[tokio::test]
async fn test_haul_refresh_with_no_data_succeeds_and_returns_miss_on_subsequent_request() {
    test(|helper| async move {
        helper.cache.refresh().await.unwrap();

        let cache_result = helper
            .cache
            .hauls_matrix(&HaulsMatrixQuery {
                months: None,
                catch_locations: None,
                gear_group_ids: None,
                species_group_ids: None,
                vessel_length_groups: None,
                vessel_ids: None,
                active_filter: ActiveHaulsFilter::Date,
                bycatch_percentage: None,
                majority_species_group: false,
            })
            .await
            .unwrap();

        assert!(cache_result.is_none());
    })
    .await;
}
#[tokio::test]
async fn test_haul_returns_hit_after_refreshing_with_data() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let mut ers_dca = ErsDca::test_default(1, Some(vessel_id));
        ers_dca.start_latitude = Some(70.536);
        ers_dca.start_longitude = Some(21.957);
        helper.db.add_ers_dca_value(ers_dca.clone()).await;

        helper.cache.refresh().await.unwrap();

        let cache_result = helper
            .cache
            .hauls_matrix(&HaulsMatrixQuery {
                months: None,
                catch_locations: None,
                gear_group_ids: None,
                species_group_ids: None,
                vessel_length_groups: None,
                vessel_ids: None,
                active_filter: ActiveHaulsFilter::Date,
                bycatch_percentage: None,
                majority_species_group: false,
            })
            .await
            .unwrap();

        assert!(cache_result.is_some());
    })
    .await;
}

#[tokio::test]
async fn test_landing_refresh_with_no_data_succeeds_and_returns_miss_on_subsequent_request() {
    test(|helper| async move {
        helper.cache.refresh().await.unwrap();

        let cache_result = helper
            .cache
            .landing_matrix(&LandingMatrixQuery {
                months: None,
                catch_locations: None,
                gear_group_ids: None,
                species_group_ids: None,
                vessel_length_groups: None,
                vessel_ids: None,
                active_filter: ActiveLandingFilter::Date,
            })
            .await
            .unwrap();

        assert!(cache_result.is_none());
    })
    .await;
}
#[tokio::test]
async fn test_landing_returns_hit_after_refreshing_with_data() {
    test(|helper| async move {
        let mut landing = Landing::test_default(1, None);
        landing.landing_timestamp = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).unwrap();

        helper.db.add_landings(vec![landing.clone()]).await;

        helper.cache.refresh().await.unwrap();

        let cache_result = helper
            .cache
            .landing_matrix(&LandingMatrixQuery {
                months: None,
                catch_locations: None,
                gear_group_ids: None,
                species_group_ids: None,
                vessel_length_groups: None,
                vessel_ids: None,
                active_filter: ActiveLandingFilter::Date,
            })
            .await
            .unwrap();

        assert!(cache_result.is_some());
    })
    .await;
}

#[tokio::test]
async fn test_landings_refreshes_changed_landing() {
    test(|helper| async move {
        let mut landing = Landing::test_default(1, None);
        landing.landing_timestamp = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).unwrap();
        helper.db.add_landings(vec![landing.clone()]).await;

        helper.cache.refresh().await.unwrap();

        landing.product.living_weight = Some(1000000000.0);
        landing.document_info.version_number += 1;
        helper.db.add_landings(vec![landing]).await;

        dbg!("going");
        helper.cache.refresh().await.unwrap();

        let cache_result = helper
            .cache
            .landing_matrix(&LandingMatrixQuery {
                months: None,
                catch_locations: None,
                gear_group_ids: None,
                species_group_ids: None,
                vessel_length_groups: None,
                vessel_ids: None,
                active_filter: ActiveLandingFilter::Date,
            })
            .await
            .unwrap()
            .unwrap();

        dbg!(&cache_result.dates);
        assert!(cache_result.dates.contains(&1000000000));
    })
    .await;
}

#[tokio::test]
async fn test_hauls_refreshes_changed_hauls() {
    test(|helper| async move {
        let vessel_id = FiskeridirVesselId::test_new(1);

        helper
            .db
            .generate_fiskeridir_vessel(vessel_id, None, None)
            .await;

        let mut ers_dca = ErsDca::test_default(1, Some(vessel_id));
        ers_dca.start_latitude = Some(70.536);
        ers_dca.start_longitude = Some(21.957);
        helper.db.add_ers_dca_value(ers_dca.clone()).await;

        helper.cache.refresh().await.unwrap();

        let expected_total = ers_dca.catch.species.living_weight.unwrap() + 20;

        let mut ers_dca = ErsDca::test_default(2, Some(vessel_id));
        ers_dca.start_latitude = Some(70.536);
        ers_dca.start_longitude = Some(21.957);
        ers_dca.catch.species.living_weight = Some(20);
        helper.db.add_ers_dca_value(ers_dca).await;

        helper.cache.refresh().await.unwrap();

        let cache_result = helper
            .cache
            .hauls_matrix(&HaulsMatrixQuery {
                months: None,
                catch_locations: None,
                gear_group_ids: None,
                species_group_ids: None,
                vessel_length_groups: None,
                vessel_ids: None,
                active_filter: ActiveHaulsFilter::Date,
                bycatch_percentage: None,
                majority_species_group: false,
            })
            .await
            .unwrap()
            .unwrap();

        assert!(cache_result.dates.contains(&(expected_total as u64)));
    })
    .await;
}
