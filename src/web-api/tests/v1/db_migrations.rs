use crate::v1::helper::test_with_master_db;
use chrono::{TimeZone, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{ActiveHaulsFilter, ActiveLandingFilter, Mmsi, TripId};
use web_api::routes::v1::{
    ais::AisTrackParameters,
    ais_vms::AisVmsParameters,
    haul::{HaulsMatrixParams, HaulsParams},
    landing::{LandingMatrixParams, LandingsParams},
    trip::TripsParameters,
};

#[tokio::test]
async fn test_new_migrations_succeed_on_old_data() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;
    })
    .await;
}

#[tokio::test]
async fn test_existing_trips_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let trips = helper
            .app
            .get_trips(TripsParameters::default())
            .await
            .unwrap();

        assert_eq!(trips.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_existing_vessels_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let vessels = helper.app.get_vessels().await.unwrap();
        assert_eq!(vessels.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_existing_hauls_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let hauls = helper.app.get_hauls(HaulsParams::default()).await.unwrap();
        assert_eq!(hauls.len(), 6);
    })
    .await;
}

#[tokio::test]
async fn test_existing_ais_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let ais = helper
            .app
            .get_ais_track(
                Mmsi::test_new(1),
                AisTrackParameters {
                    start: Some(Utc.with_ymd_and_hms(2010, 2, 5, 10, 0, 0).unwrap()),
                    end: Some(Utc.with_ymd_and_hms(2011, 2, 5, 10, 0, 0).unwrap()),
                },
            )
            .await
            .unwrap();
        assert_eq!(ais.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn test_existing_ais_vms_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let ais = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: Some(Mmsi::test_new(1)),
                call_sign: Some(CallSign::try_from("CS1").unwrap()),
                trip_id: None,
                start: Some(Utc.with_ymd_and_hms(2010, 2, 5, 10, 0, 0).unwrap()),
                end: Some(Utc.with_ymd_and_hms(2011, 2, 5, 10, 0, 0).unwrap()),
            })
            .await
            .unwrap();
        assert_eq!(ais.len(), 5);
    })
    .await;
}

#[tokio::test]
async fn test_existing_ais_vms_by_trip_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let ais = helper
            .app
            .get_ais_vms_positions(AisVmsParameters {
                mmsi: Some(Mmsi::test_new(1)),
                call_sign: None,
                trip_id: Some(TripId::test_new(1)),
                start: None,
                end: None,
            })
            .await
            .unwrap();
        assert_eq!(ais.len(), 5);
    })
    .await;
}

#[tokio::test]
async fn test_existing_landings_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let landings = helper
            .app
            .get_landings(LandingsParams::default())
            .await
            .unwrap();
        assert_eq!(landings.len(), 6);
    })
    .await;
}

#[tokio::test]
async fn test_existing_landing_matrix_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let matrix = helper
            .app
            .get_landing_matrix(LandingMatrixParams::default(), ActiveLandingFilter::Date)
            .await
            .unwrap();
        assert!(!matrix.dates.is_empty());
        assert!(!matrix.length_group.is_empty());
        assert!(!matrix.gear_group.is_empty());
        assert!(!matrix.species_group.is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_existing_haul_matrix_succeeds_with_new_migration() {
    test_with_master_db(|helper, _builder| async move {
        helper.run_new_migrations().await;

        let matrix = helper
            .app
            .get_hauls_matrix(HaulsMatrixParams::default(), ActiveHaulsFilter::Date)
            .await
            .unwrap();
        assert!(!matrix.dates.is_empty());
        assert!(!matrix.length_group.is_empty());
        assert!(!matrix.gear_group.is_empty());
        assert!(!matrix.species_group.is_empty());
    })
    .await;
}
