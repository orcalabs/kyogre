use enum_index::EnumIndex;
use kyogre_core::{date_feature_matrix_index, NUM_CATCH_LOCATIONS};

use crate::v1::helper::{assert_matrix_content, sum_area};

use super::helper::test;
use actix_web::http::StatusCode;
use chrono::{DateTime, Utc};
use fiskeridir_rs::{ErsDca, GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{ActiveHaulsFilter, FiskeridirVesselId, ScraperInboundPort};
use web_api::routes::{
    utils::{self, DateTimeUtc, GearGroupId, SpeciesGroupId},
    v1::haul::{Haul, HaulsMatrix, HaulsMatrixParams, HaulsParams},
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

        ers1.set_start_timestamp(month1);
        ers1.set_stop_timestamp(month1);
        ers2.set_start_timestamp(month2);
        ers2.set_stop_timestamp(month2);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

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

#[tokio::test]
async fn test_hauls_returns_hauls_with_gear_group_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.gear.gear_group_code = GearGroup::Not;
        ers2.gear.gear_group_code = GearGroup::BurOgRuser;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::BurOgRuser),
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

#[tokio::test]
async fn test_hauls_returns_hauls_with_species_group_ids() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers2.catch.species.species_group_code = SpeciesGroup::Uer;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsParams {
            species_group_ids: Some(vec![SpeciesGroupId(301), SpeciesGroupId(302)]),
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
async fn test_hauls_returns_hauls_with_vessel_length_ranges() {
    test(|helper| async move {
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers2.vessel_info.vessel_length = 12.;

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsParams {
            vessel_length_ranges: Some(vec!["(,10)".parse().unwrap(), "[10,15)".parse().unwrap()]),
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
async fn test_hauls_returns_hauls_with_fiskeridir_vessel_ids() {
    test(|helper| async move {
        let ers1 = ErsDca::test_default(1, Some(1));
        let ers2 = ErsDca::test_default(2, Some(2));
        let ers3 = ErsDca::test_default(3, None);
        let ers4 = ErsDca::test_default(4, None);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsParams {
            fiskeridir_vessel_ids: Some(vec![FiskeridirVesselId(1), FiskeridirVesselId(2)]),
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
async fn test_hauls_matrix_returns_correct_sum_for_all_hauls() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;
        let mut ers1 = ErsDca::test_default(1, Some(1));
        let mut ers2 = ErsDca::test_default(2, Some(2));

        ers1.catch.species.living_weight = Some(20);
        ers2.catch.species.living_weight = Some(40);

        ers1.start_latitude = Some(70.536);
        ers1.start_longitude = Some(21.957);
        ers2.start_latitude = Some(70.536);
        ers2.start_longitude = Some(21.957);

        helper.db.db.add_ers_dca(vec![ers1, ers2]).await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        assert_matrix_content(&matrix, filter, 60);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_months() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        ers1.set_start_timestamp(month1);
        ers1.set_stop_timestamp(month1);
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.set_start_timestamp(month2);
        ers2.set_stop_timestamp(month2);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsMatrixParams {
            months: Some(vec![month1.into(), month2.into()]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_vessel_length() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::VesselLength;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.vessel_info.vessel_length = 12.;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsMatrixParams {
            vessel_length_groups: Some(vec![
                utils::VesselLengthGroup(VesselLengthGroup::UnderEleven),
                utils::VesselLengthGroup(VesselLengthGroup::ElevenToFifteen),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_species_group() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.catch.species.species_group_code = SpeciesGroup::Uer;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsMatrixParams {
            species_group_ids: Some(vec![SpeciesGroupId(301), SpeciesGroupId(302)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_gear_group() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::GearGroup;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.gear.gear_group_code = GearGroup::Not;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.gear.gear_group_code = GearGroup::Garn;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsMatrixParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::Garn),
            ]),
            ..Default::default()
        };

        let response = helper
            .app
            .get_hauls_matrix(params, ActiveHaulsFilter::GearGroup)
            .await;

        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_fiskeridir_vessel_ids() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;

        let mut ers1 = ErsDca::test_default(1, Some(1));
        let mut ers2 = ErsDca::test_default(2, Some(2));
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let params = HaulsMatrixParams {
            fiskeridir_vessel_ids: Some(vec![FiskeridirVesselId(1), FiskeridirVesselId(2)]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_date_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::Date;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        ers1.set_start_timestamp(month1);
        ers1.set_stop_timestamp(month1);
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.set_start_timestamp(month2);
        ers2.set_stop_timestamp(month2);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = date_feature_matrix_index(&month1);
        let y1 = date_feature_matrix_index(&month2);

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.dates, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_gear_group_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::GearGroup;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.gear.gear_group_code = GearGroup::Traal;
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.gear.gear_group_code = GearGroup::Snurrevad;
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.gear.gear_group_code = GearGroup::Not;
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.gear.gear_group_code = GearGroup::Not;
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = GearGroup::Traal.enum_index();
        let y1 = GearGroup::Snurrevad.enum_index();

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.gear_group, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_vessel_length_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::VesselLength;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.vessel_info.vessel_length = 9.;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.vessel_info.vessel_length = 12.;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = VesselLengthGroup::UnderEleven.enum_index();
        let y1 = VesselLengthGroup::ElevenToFifteen.enum_index();

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.length_group, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_species_group_sum_area_table_is_correct() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;
        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        ers1.catch.species.species_group_code = SpeciesGroup::Blaakveite;
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.catch.species.species_group_code = SpeciesGroup::Uer;
        ers2.start_latitude = Some(56.727258);
        ers2.start_longitude = Some(12.565410);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = NUM_CATCH_LOCATIONS;
        let x0 = 0;
        let x1 = width - 1;
        let y0 = SpeciesGroup::Blaakveite.enum_index();
        let y1 = SpeciesGroup::Uer.enum_index();

        assert_matrix_content(&matrix, filter, 330);
        assert_eq!(sum_area(&matrix.species_group, x0, y0, x1, y1, width), 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_catch_location_as_active_filter_produces_correct_matrices() {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::CatchLocation;

        let mut ers1 = ErsDca::test_default(1, None);
        let mut ers2 = ErsDca::test_default(2, None);
        let mut ers3 = ErsDca::test_default(3, None);
        let mut ers4 = ErsDca::test_default(4, None);

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        ers1.set_start_timestamp(month1);
        ers1.set_stop_timestamp(month1);
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        ers2.start_latitude = Some(56.756293);
        ers2.start_longitude = Some(11.514740);
        ers2.set_start_timestamp(month2);
        ers2.set_stop_timestamp(month2);
        ers2.catch.species.living_weight = Some(20);

        ers3.start_latitude = Some(56.727258);
        ers3.start_longitude = Some(12.565410);
        ers3.catch.species.living_weight = Some(100);
        ers4.start_latitude = Some(56.727258);
        ers4.start_longitude = Some(12.565410);
        ers4.catch.species.living_weight = Some(200);

        helper
            .db
            .db
            .add_ers_dca(vec![ers1, ers2, ers3, ers4])
            .await
            .unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 330);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_have_correct_totals_after_dca_message_is_replaced_by_newer_version_with_another_weight(
) {
    test(|helper| async move {
        let filter = ActiveHaulsFilter::CatchLocation;

        let mut ers1 = ErsDca::test_default(1, None);

        let date: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        ers1.set_start_timestamp(date);
        ers1.set_stop_timestamp(date);
        ers1.start_latitude = Some(56.727258);
        ers1.start_longitude = Some(12.565410);
        ers1.catch.species.living_weight = Some(10);

        let mut ers2 = ers1.clone();
        ers2.message_version = ers1.message_version + 1;
        ers2.catch.species.living_weight = Some(20);

        helper.db.db.add_ers_dca(vec![ers1, ers2]).await.unwrap();

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_matrix_content(&matrix, filter, 20);
    })
    .await
}
