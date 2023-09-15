use super::helper::test_with_cache;
use crate::v1::helper::*;
use actix_web::http::StatusCode;
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use enum_index::EnumIndex;
use fiskeridir_rs::{GearGroup, Landing, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    landing_date_feature_matrix_index, levels::*, ActiveLandingFilter, CatchLocationId,
    HaulMatrixes, LandingMatrixes, NUM_CATCH_LOCATIONS,
};
use web_api::routes::utils::{self, GearGroupId, SpeciesGroupId};
use web_api::routes::v1::landing::{LandingMatrix, LandingMatrixParams};

#[tokio::test]
async fn test_landing_matrix_returns_correct_sum_for_all_landings() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::Date;

        builder
            .landings(2)
            .modify_idx(|i, v| match i {
                0 => v.product.living_weight = Some(20.0),
                1 => v.product.living_weight = Some(40.0),
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();

        assert_landing_matrix_content(&matrix, filter, 60, vec![]);
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_filters_by_catch_locations() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::GearGroup;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.catch_location.main_area_code = Some(9);
                    v.catch_location.location_code = Some(32);
                }
                1 => v.product.living_weight = Some(40.0),
                2 => v.product.living_weight = Some(100.0),
                _ => (),
            })
            .build()
            .await;

        let params = LandingMatrixParams {
            catch_locations: Some(vec![CatchLocationId::new(9, 32)]),
            ..Default::default()
        };

        helper.refresh_cache().await;

        let response = helper.app.get_landing_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(&matrix, filter, 20, vec![(LandingMatrixes::GearGroup, 160)]);
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_filters_by_months() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::GearGroup;

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.landing_timestamp = month1;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.landing_timestamp = month2;
                }
                2 => v.product.living_weight = Some(100.0),
                _ => (),
            })
            .build()
            .await;

        let params = LandingMatrixParams {
            months: Some(vec![month1.into(), month2.into()]),
            ..Default::default()
        };

        helper.refresh_cache().await;

        let response = helper.app.get_landing_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(&matrix, filter, 60, vec![(LandingMatrixes::Date, 160)]);
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_filters_by_vessel_length() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::SpeciesGroup;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.vessel.length_group_code = VesselLengthGroup::UnderEleven;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.vessel.length_group_code = VesselLengthGroup::ElevenToFifteen;
                }
                2 => {
                    v.product.living_weight = Some(100.0);
                    v.vessel.length_group_code = VesselLengthGroup::TwentyTwoToTwentyEight;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;
        let params = LandingMatrixParams {
            vessel_length_groups: Some(vec![
                utils::VesselLengthGroup(VesselLengthGroup::UnderEleven),
                utils::VesselLengthGroup(VesselLengthGroup::ElevenToFifteen),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_landing_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(
            &matrix,
            filter,
            60,
            vec![(LandingMatrixes::VesselLength, 160)],
        );
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_filters_by_species_group() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::GearGroup;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.vessel.length_group_code = VesselLengthGroup::UnderEleven;
                    v.product.species.group_code = SpeciesGroup::Blaakveite;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.vessel.length_group_code = VesselLengthGroup::ElevenToFifteen;
                    v.product.species.group_code = SpeciesGroup::Uer;
                }
                2 => {
                    v.product.living_weight = Some(100.0);
                    v.product.species.group_code = SpeciesGroup::Sei;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;
        let params = LandingMatrixParams {
            species_group_ids: Some(vec![
                SpeciesGroupId(SpeciesGroup::Uer),
                SpeciesGroupId(SpeciesGroup::Blaakveite),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_landing_matrix(params, filter).await;

        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(
            &matrix,
            filter,
            60,
            vec![(LandingMatrixes::SpeciesGroup, 160)],
        );
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_filters_by_gear_group() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::SpeciesGroup;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.gear.group = GearGroup::Not;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.gear.group = GearGroup::Garn;
                }
                2 => {
                    v.product.living_weight = Some(100.0);
                    v.gear.group = GearGroup::Oppdrett;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;
        let params = LandingMatrixParams {
            gear_group_ids: Some(vec![
                GearGroupId(GearGroup::Not),
                GearGroupId(GearGroup::Garn),
            ]),
            ..Default::default()
        };

        let response = helper.app.get_landing_matrix(params, filter).await;

        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(&matrix, filter, 60, vec![(LandingMatrixes::GearGroup, 160)]);
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_filters_by_fiskeridir_vessel_ids() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::Date;

        let state = builder
            .landings(1)
            .vessels(2)
            .landings(2)
            .modify(|v| v.product.living_weight = Some(30.0))
            .build()
            .await;

        helper.refresh_cache().await;
        let params = LandingMatrixParams {
            fiskeridir_vessel_ids: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
            ..Default::default()
        };

        let response = helper.app.get_landing_matrix(params, filter).await;

        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(&matrix, filter, 60, vec![]);
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_date_sum_area_table_is_correct() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::Date;

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.landing_timestamp = month1;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.landing_timestamp = month2;
                }
                2 => v.product.living_weight = Some(100.0),
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();

        let width = LandingMatrixes::Date.size();
        let x0 = landing_date_feature_matrix_index(&month1);
        let x1 = landing_date_feature_matrix_index(&month2);
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_landing_matrix_content(&matrix, filter, 160, vec![]);
        assert_eq!(sum_area(&matrix.dates, x0, y0, x1, y1, width), 60);
    })
    .await
}

#[tokio::test]
async fn test_landing_matrix_gear_group_sum_area_table_is_correct() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::GearGroup;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.gear.group = GearGroup::Traal;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.gear.group = GearGroup::Snurrevad;
                }
                2 => {
                    v.product.living_weight = Some(100.0);
                    v.gear.group = GearGroup::Not;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();

        let width = LandingMatrixes::GearGroup.size();
        let x0 = GearGroup::Traal.enum_index();
        let x1 = GearGroup::Snurrevad.enum_index();
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_landing_matrix_content(&matrix, filter, 160, vec![]);
        assert_eq!(sum_area(&matrix.gear_group, x0, y0, x1, y1, width), 60);
    })
    .await
}

#[tokio::test]
async fn test_landing_matrix_vessel_length_sum_area_table_is_correct() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::VesselLength;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.vessel.length_group_code = VesselLengthGroup::UnderEleven;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.vessel.length_group_code = VesselLengthGroup::ElevenToFifteen;
                }
                2 => {
                    v.product.living_weight = Some(100.0);
                    v.vessel.length_group_code = VesselLengthGroup::TwentyTwoToTwentyEight;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();

        let width = HaulMatrixes::VesselLength.size();
        let x0 = VesselLengthGroup::UnderEleven.enum_index();
        let x1 = VesselLengthGroup::ElevenToFifteen.enum_index();
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_landing_matrix_content(&matrix, filter, 160, vec![]);
        assert_eq!(sum_area(&matrix.length_group, x0, y0, x1, y1, width), 60);
    })
    .await
}

#[tokio::test]
async fn test_landing_matrix_species_group_sum_area_table_is_correct() {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::SpeciesGroup;

        builder
            .landings(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.product.living_weight = Some(20.0);
                    v.vessel.length_group_code = VesselLengthGroup::UnderEleven;
                    v.product.species.group_code = SpeciesGroup::Blaakveite;
                }
                1 => {
                    v.product.living_weight = Some(40.0);
                    v.vessel.length_group_code = VesselLengthGroup::ElevenToFifteen;
                    v.product.species.group_code = SpeciesGroup::Uer;
                }
                2 => {
                    v.product.living_weight = Some(100.0);
                    v.product.species.group_code = SpeciesGroup::Sei;
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_cache().await;

        let response = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();

        let width = LandingMatrixes::SpeciesGroup.size();
        let x0 = SpeciesGroup::Blaakveite.enum_index();
        let x1 = SpeciesGroup::Uer.enum_index();
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_landing_matrix_content(&matrix, filter, 160, vec![]);
        assert_eq!(sum_area(&matrix.species_group, x0, y0, x1, y1, width), 60);
    })
    .await;
}

#[tokio::test]
async fn test_landing_matrix_have_correct_totals_after_landing_is_replaced_by_newer_version_with_another_weight(
) {
    test_with_cache(|helper, builder| async move {
        let filter = ActiveLandingFilter::SpeciesGroup;

        let mut landing = Landing::test_default(1, None);
        landing.landing_timestamp = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).unwrap();
        landing.product.living_weight = Some(20.0);

        helper.db.add_landings(vec![landing.clone()]).await;

        let mut landing2 = landing.clone();
        landing2.landing_timestamp = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).unwrap();
        landing2.product.living_weight = Some(40.0);
        landing2.document_info.version_number += 1;

        helper.db.add_landings(vec![landing2]).await;

        builder
            .landings(1)
            .modify(|v| {
                v.product.living_weight = Some(20.0);
                v.id = LandingId::try_from("1-7-0").unwrap();
            })
            .build()
            .await;

        helper
            .builder()
            .await
            .landings(1)
            .modify(|v| {
                v.product.living_weight = Some(40.0);
                v.id = LandingId::try_from("1-7-0").unwrap();
                v.document_info.version_number += 1;
            })
            .build()
            .await;

        helper.refresh_cache().await;
        let response = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: LandingMatrix = response.json().await.unwrap();
        assert_landing_matrix_content(&matrix, filter, 40, vec![]);
    })
    .await
}
