use super::helper::test;
use actix_web::http::StatusCode;
use fiskeridir_rs::{CallSign, ErsDca, Landing};
use kyogre_core::{FiskeridirVesselId, Mmsi, ScraperInboundPort};
use web_api::routes::v1::vessel::Vessel;

#[tokio::test]
async fn test_vessels_returns_merged_data_from_fiskeridir_and_ais_and_ers() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("LK-28").unwrap();
        let ais_vessel = helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;

        let vessel_id = 1;
        let mut landing = Landing::test_default(1, Some(vessel_id));
        landing.vessel.call_sign = Some(call_sign.clone());

        let mut ers = ErsDca::test_default(1, Some(vessel_id as u64));
        ers.vessel_info.call_sign_ers = call_sign.into_inner().try_into().unwrap();

        helper
            .handle()
            .add_landings(vec![landing.clone()])
            .await
            .unwrap();

        helper.db.add_ers_dca(vec![ers.clone()]).await;

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        let received_vessel = body.pop().unwrap();

        assert_eq!(received_vessel.fiskeridir.unwrap(), landing.vessel);
        assert_eq!(received_vessel.ais.unwrap(), ais_vessel);
        assert_eq!(received_vessel.ers.unwrap(), ers.vessel_info);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_identification_does_not_overwrite_fields() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("LK-28").unwrap();

        let vessel_id = 1;
        let mut landing = Landing::test_default(1, Some(vessel_id));
        landing.vessel.call_sign = Some(call_sign.clone());

        let mut ers = ErsDca::test_default(1, Some(vessel_id as u64));
        ers.vessel_info.call_sign_ers = call_sign.clone().into_inner().try_into().unwrap();

        helper
            .handle()
            .add_landings(vec![landing.clone()])
            .await
            .unwrap();

        helper.db.add_ers_dca(vec![ers.clone()]).await;

        let call_sign2 = "Other CallSign".to_string();
        let mut ers2 = ErsDca::test_default(2, Some(vessel_id as u64));
        ers2.vessel_info.call_sign_ers = call_sign2.clone().try_into().unwrap();

        helper.db.add_ers_dca(vec![ers2]).await;

        let response = helper.app.get_vessels().await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        let received_vessel = body.pop().unwrap();

        assert_eq!(received_vessel.fiskeridir.unwrap(), landing.vessel);
        assert_eq!(received_vessel.ers.unwrap(), ers.vessel_info);

        let conflicts = helper.db.vessel_identification_conflicts().await;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].old_value, call_sign.into_inner());
        assert_eq!(conflicts[0].new_value, call_sign2);
    })
    .await;
}

#[tokio::test]
async fn test_vessel_identification_handles_merging_two_vessels() {
    test(|helper| async move {
        let fiskeridir_vessel_id = FiskeridirVesselId(11);
        let mut landing = Landing::test_default(1, Some(fiskeridir_vessel_id.0));
        landing.vessel.call_sign = None;

        helper
            .handle()
            .add_landings(vec![landing.clone()])
            .await
            .unwrap();

        let call_sign = "LK28".to_string();
        let ais_vessel = helper.db.generate_ais_vessel(Mmsi(40), &call_sign).await;

        let response = helper.app.get_vessels().await;
        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();
        assert_eq!(vessels.len(), 2);

        let mut ers = ErsDca::test_default(2, Some(fiskeridir_vessel_id.0 as u64));
        ers.vessel_info.call_sign_ers = call_sign.clone().try_into().unwrap();
        helper.db.add_ers_dca(vec![ers]).await;

        let response = helper.app.get_vessels().await;
        assert_eq!(response.status(), StatusCode::OK);
        let vessels: Vec<Vessel> = response.json().await.unwrap();

        assert_eq!(vessels.len(), 1);
        assert_eq!(vessels[0].call_sign, Some(call_sign.clone()));
        assert_eq!(
            vessels[0].fiskeridir.as_ref().unwrap().id,
            fiskeridir_vessel_id
        );
        assert_eq!(vessels[0].ais.as_ref().unwrap().mmsi, ais_vessel.mmsi);
        assert_eq!(vessels[0].ers.as_ref().unwrap().call_sign, call_sign);
    })
    .await;
}
