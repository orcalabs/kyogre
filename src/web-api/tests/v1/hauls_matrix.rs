use super::helper::test_with_matrix_cache;
use crate::v1::helper::test;
use crate::v1::helper::{assert_haul_matrix_content, sum_area};
use actix_web::http::StatusCode;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use engine::*;
use enum_index::EnumIndex;
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{
    haul_date_feature_matrix_index, ActiveHaulsFilter, CatchLocationId, HaulMatrixes,
    NUM_CATCH_LOCATIONS,
};
use web_api::routes::{
    utils::datetime_to_month,
    v1::haul::{HaulsMatrix, HaulsMatrixParams},
};

#[tokio::test]
async fn test_hauls_matrix_does_not_query_database_on_prior_month_or_newer_data() {
    test(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;
        let now = Utc::now();

        let (prior_year, prior_month) = if now.month() == 1 {
            (now.year() - 1, 12)
        } else {
            (now.year(), now.month() - 1)
        };

        let (next_year, next_month) = if now.month() == 12 {
            (now.year() + 1, 1)
        } else {
            (now.year(), now.month() + 1)
        };

        let prior_month = Utc
            .with_ymd_and_hms(prior_year, prior_month, 1, 0, 0, 0)
            .unwrap();
        let next_month = Utc
            .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
            .unwrap();

        builder
            .vessels(1)
            .hauls(3)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.set_start_timestamp(prior_month);
                    v.dca.set_stop_timestamp(prior_month + Duration::hours(1));
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(100);
                    v.dca.catch.species.species_group_code = SpeciesGroup::AtlanticCod;
                    v.dca.catch.species.species_fao_code = Some("test".into());
                }
                1 => {
                    v.dca.set_start_timestamp(now);
                    v.dca.set_stop_timestamp(now + Duration::hours(1));
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(90);
                    v.dca.catch.species.species_group_code = SpeciesGroup::GoldenRedfish;
                    v.dca.catch.species.species_fao_code = Some("test2".into());
                }
                2 => {
                    v.dca.set_start_timestamp(next_month);
                    v.dca.set_stop_timestamp(next_month + Duration::hours(1));
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(90);
                    v.dca.catch.species.species_group_code = SpeciesGroup::GoldenRedfish;
                    v.dca.catch.species.species_fao_code = Some("test2".into());
                }
                _ => (),
            })
            .build()
            .await;

        let cutoff_1 = prior_month.year() as u32 * 12 + prior_month.month0();
        let cutoff_2 = now.year() as u32 * 12 + now.month0();
        let cutoff_3 = next_month.year() as u32 * 12 + next_month.month0();
        assert_eq!(cutoff_1, cutoff_2 - 1);
        assert_eq!(cutoff_3, cutoff_2 + 1);

        let response = helper
            .app
            .get_hauls_matrix(
                HaulsMatrixParams {
                    months: Some(vec![cutoff_1, cutoff_2, cutoff_3]),
                    ..Default::default()
                },
                filter,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert!(matrix.dates.is_empty());
        assert!(matrix.length_group.is_empty());
        assert!(matrix.gear_group.is_empty());
        assert!(matrix.species_group.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_majority_species() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;
        let start = Utc::now();
        let end = start + Duration::hours(1);
        builder
            .vessels(2)
            .hauls(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.message_info.message_id = 1;
                    v.dca.set_start_timestamp(start);
                    v.dca.set_stop_timestamp(end);
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(100);
                    v.dca.catch.species.species_group_code = SpeciesGroup::AtlanticCod;
                    v.dca.catch.species.species_fao_code = Some("test".into());
                }
                1 => {
                    v.dca.message_info.message_id = 1;
                    v.dca.set_start_timestamp(start);
                    v.dca.set_stop_timestamp(end);
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(90);
                    v.dca.catch.species.species_group_code = SpeciesGroup::GoldenRedfish;
                    v.dca.catch.species.species_fao_code = Some("test2".into());
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;

        let response = helper
            .app
            .get_hauls_matrix(
                HaulsMatrixParams {
                    majority_species_group: Some(true),
                    bycatch_percentage: Some(0.1),
                    ..Default::default()
                },
                filter,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        assert_haul_matrix_content(&matrix, filter, 100, vec![]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_bycatch() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;
        let start = Utc::now();
        let end = start + Duration::hours(1);
        builder
            .vessels(2)
            .hauls(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.message_info.message_id = 1;
                    v.dca.set_start_timestamp(start);
                    v.dca.set_stop_timestamp(end);
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(100);
                    v.dca.catch.species.species_group_code = SpeciesGroup::AtlanticCod;
                    v.dca.catch.species.species_fao_code = Some("test".into());
                }
                1 => {
                    v.dca.message_info.message_id = 1;
                    v.dca.set_start_timestamp(start);
                    v.dca.set_stop_timestamp(end);
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(10);
                    v.dca.catch.species.species_group_code = SpeciesGroup::GoldenRedfish;
                    v.dca.catch.species.species_fao_code = Some("test2".into());
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;

        let response = helper
            .app
            .get_hauls_matrix(
                HaulsMatrixParams {
                    bycatch_percentage: Some(15.0),
                    ..Default::default()
                },
                filter,
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        assert_haul_matrix_content(&matrix, filter, 100, vec![]);
    })
    .await;
}
#[tokio::test]
async fn test_hauls_matrix_returns_correct_sum_for_all_hauls() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;
        builder
            .vessels(2)
            .hauls(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(20);
                }
                1 => {
                    v.dca.start_latitude = Some(70.536);
                    v.dca.start_longitude = Some(21.957);
                    v.dca.catch.species.living_weight = Some(40);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;

        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        assert_haul_matrix_content(&matrix, filter, 60, vec![]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_months() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::GearGroup;

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.set_start_timestamp(month1);
                    v.dca.set_stop_timestamp(month1);
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.start_latitude = Some(56.756293);
                    v.dca.start_longitude = Some(11.514740);
                    v.dca.set_start_timestamp(month2);
                    v.dca.set_stop_timestamp(month2);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        let params = HaulsMatrixParams {
            months: Some(vec![datetime_to_month(month1), datetime_to_month(month2)]),
            ..Default::default()
        };

        helper.refresh_matrix_cache().await;

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 30, vec![(HaulMatrixes::Date, 330)]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_vessel_length() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.vessel_info.vessel_length = 9.;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.vessel_info.vessel_length = 12.;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let params = HaulsMatrixParams {
            vessel_length_groups: Some(vec![
                VesselLengthGroup::UnderEleven,
                VesselLengthGroup::ElevenToFifteen,
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 30, vec![(HaulMatrixes::VesselLength, 330)]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_species_group() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::GearGroup;

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.catch.species.species_group_code = SpeciesGroup::GreenlandHalibut;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.catch.species.species_group_code = SpeciesGroup::GoldenRedfish;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let params = HaulsMatrixParams {
            species_group_ids: Some(vec![
                SpeciesGroup::GoldenRedfish,
                SpeciesGroup::GreenlandHalibut,
            ]),
            ..Default::default()
        };

        let response = helper.app.get_hauls_matrix(params, filter).await;

        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 30, vec![(HaulMatrixes::SpeciesGroup, 330)]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_gear_group() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.gear.gear_group_code = GearGroup::Seine;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.gear.gear_group_code = GearGroup::Net;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        let params = HaulsMatrixParams {
            gear_group_ids: Some(vec![GearGroup::Seine, GearGroup::Net]),
            ..Default::default()
        };

        helper.refresh_matrix_cache().await;
        let response = helper.app.get_hauls_matrix(params, filter).await;

        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 30, vec![(HaulMatrixes::GearGroup, 330)]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_fiskeridir_vessel_ids() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;

        let state = builder
            .hauls(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                1 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .vessels(2)
            .hauls(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.start_latitude = Some(56.756293);
                    v.dca.start_longitude = Some(11.514740);
                    v.dca.catch.species.living_weight = Some(20);
                }
                _ => (),
            })
            .build()
            .await;

        let params = HaulsMatrixParams {
            fiskeridir_vessel_ids: Some(state.vessels.iter().map(|v| v.fiskeridir.id).collect()),
            ..Default::default()
        };

        helper.refresh_matrix_cache().await;
        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 30, vec![]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_filters_by_catch_locations() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;

        builder
            .hauls(2)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.start_latitude = Some(67.125);
                    v.dca.start_longitude = Some(13.5);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.catch.species.living_weight = Some(20);
                    v.dca.start_latitude = Some(67.5);
                    v.dca.start_longitude = Some(43.5);
                }
                _ => (),
            })
            .build()
            .await;

        let params = HaulsMatrixParams {
            catch_locations: Some(vec![CatchLocationId::new(0, 5)]),
            ..Default::default()
        };

        helper.refresh_matrix_cache().await;
        let response = helper.app.get_hauls_matrix(params, filter).await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 10, vec![(HaulMatrixes::Date, 30)]);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_date_sum_area_table_is_correct() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::Date;

        let month1: DateTime<Utc> = "2013-01-1T00:00:00Z".parse().unwrap();
        let month2: DateTime<Utc> = "2013-06-1T00:00:00Z".parse().unwrap();

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.set_start_timestamp(month1);
                    v.dca.set_stop_timestamp(month1);
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.start_latitude = Some(56.756293);
                    v.dca.start_longitude = Some(11.514740);
                    v.dca.set_start_timestamp(month2);
                    v.dca.set_stop_timestamp(month2);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = HaulMatrixes::Date.size();
        let x0 = haul_date_feature_matrix_index(&month1);
        let x1 = haul_date_feature_matrix_index(&month2);
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_haul_matrix_content(&matrix, filter, 330, vec![]);
        assert_eq!(sum_area(&matrix.dates, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_gear_group_sum_area_table_is_correct() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::GearGroup;

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.gear.gear_group_code = GearGroup::Trawl;
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.start_latitude = Some(56.756293);
                    v.dca.start_longitude = Some(11.514740);
                    v.dca.gear.gear_group_code = GearGroup::DanishSeine;
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.gear.gear_group_code = GearGroup::Seine;
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.gear.gear_group_code = GearGroup::Seine;
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = HaulMatrixes::GearGroup.size();
        let x0 = GearGroup::Trawl.enum_index();
        let x1 = GearGroup::DanishSeine.enum_index();
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_haul_matrix_content(&matrix, filter, 330, vec![]);
        assert_eq!(sum_area(&matrix.gear_group, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_vessel_length_sum_area_table_is_correct() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::VesselLength;

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.vessel_info.vessel_length = 9.;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.vessel_info.vessel_length = 12.;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = HaulMatrixes::VesselLength.size();
        let x0 = VesselLengthGroup::UnderEleven.enum_index();
        let x1 = VesselLengthGroup::ElevenToFifteen.enum_index();
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_haul_matrix_content(&matrix, filter, 330, vec![]);
        assert_eq!(sum_area(&matrix.length_group, x0, y0, x1, y1, width), 30);
    })
    .await
}

#[tokio::test]
async fn test_hauls_matrix_species_group_sum_area_table_is_correct() {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;

        builder
            .hauls(4)
            .modify_idx(|i, v| match i {
                0 => {
                    v.dca.catch.species.species_group_code = SpeciesGroup::GreenlandHalibut;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(10);
                }
                1 => {
                    v.dca.catch.species.species_group_code = SpeciesGroup::GoldenRedfish;
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(20);
                }
                2 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(100);
                }
                3 => {
                    v.dca.start_latitude = Some(56.727258);
                    v.dca.start_longitude = Some(12.565410);
                    v.dca.catch.species.living_weight = Some(200);
                }
                _ => (),
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();

        let width = HaulMatrixes::SpeciesGroup.size();
        let x0 = SpeciesGroup::GreenlandHalibut.enum_index();
        let x1 = SpeciesGroup::GoldenRedfish.enum_index();
        let y0 = 0;
        let y1 = NUM_CATCH_LOCATIONS - 1;

        assert_haul_matrix_content(&matrix, filter, 330, vec![]);
        assert_eq!(sum_area(&matrix.species_group, x0, y0, x1, y1, width), 30);
    })
    .await;
}

#[tokio::test]
async fn test_hauls_matrix_have_correct_totals_after_dca_message_is_replaced_by_newer_version_with_another_weight(
) {
    test_with_matrix_cache(|helper, builder| async move {
        let filter = ActiveHaulsFilter::SpeciesGroup;

        let message_id = 1;

        builder
            .hauls(1)
            .modify(|v| {
                v.dca.message_info.message_id = message_id;
                v.dca.start_latitude = Some(56.727258);
                v.dca.start_longitude = Some(12.565410);
                v.dca.catch.species.living_weight = Some(10);
            })
            .new_cycle()
            .hauls(1)
            .modify(|v| {
                v.dca.message_info.message_id = message_id;
                v.dca.message_version = 2;
                v.dca.catch.species.living_weight = Some(20);
                v.dca.start_latitude = Some(56.727258);
                v.dca.start_longitude = Some(12.565410);
            })
            .build()
            .await;

        helper.refresh_matrix_cache().await;
        let response = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let matrix: HaulsMatrix = response.json().await.unwrap();
        assert_haul_matrix_content(&matrix, filter, 20, vec![]);
    })
    .await
}
