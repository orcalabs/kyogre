use super::helper::test;
use actix_web::http::StatusCode;
use fiskeridir_rs::{CallSign, Landing};
use kyogre_core::{Mmsi, ScraperInboundPort};
use web_api::routes::v1::vessel::Vessel;

#[tokio::test]
async fn test_vessels_returns_merged_data_from_fiskeridir_and_ais() {
    test(|helper| async move {
        let call_sign = CallSign::try_from("LK-28").unwrap();
        let ais_vessel = helper
            .db
            .generate_ais_vessel(Mmsi(40), call_sign.as_ref())
            .await;

        let vessel_id = 1;
        let mut landing = Landing::test_default(1, Some(vessel_id));
        landing.vessel.call_sign = Some(call_sign);

        helper
            .handle()
            .add_landings(vec![landing.clone()], 2023)
            .await
            .unwrap();

        let response = helper.app.get_vessels().await;

        assert_eq!(response.status(), StatusCode::OK);
        let mut body: Vec<Vessel> = response.json().await.unwrap();
        let received_vessel = body.pop().unwrap();

        assert_eq!(received_vessel.fiskeridir, landing.vessel);
        assert_eq!(received_vessel.ais.unwrap(), ais_vessel);
    })
    .await;
}
