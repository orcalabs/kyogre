use crate::{
    error::{error::FuelAfterLowerThanFuelSnafu, Result},
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response, Database,
};
use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{
    CreateFuelMeasurement, DeleteFuelMeasurement, FuelMeasurement, FuelMeasurementsQuery,
};
use oasgen::{oasgen, OaSchema};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;

#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurementsParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn get_fuel_measurements<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FuelMeasurementsParams>,
) -> Result<StreamResponse<FuelMeasurement>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().to_query(call_sign.clone());

    let response = stream_response! {
        db.fuel_measurements(query)
    };

    Ok(response)
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn create_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<Vec<CreateFuelMeasurement>>,
) -> Result<Response<Vec<FuelMeasurement>>> {
    let body = body.into_inner();
    if let Some((fuel_after, fuel)) = body
        .iter()
        .filter_map(|b| b.fuel_after.map(|a| (a, b.fuel)))
        .find(|v| v.0 <= v.1)
    {
        return FuelAfterLowerThanFuelSnafu { fuel_after, fuel }.fail();
    };

    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    let measurements = db.add_fuel_measurements(&body, call_sign, user_id).await?;

    Ok(Response::new(measurements))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn update_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<Vec<FuelMeasurement>>,
) -> Result<Response<()>> {
    let body = body.into_inner();
    if let Some((fuel_after, fuel)) = body
        .iter()
        .filter_map(|b| b.fuel_after.map(|a| (a, b.fuel)))
        .find(|v| v.0 <= v.1)
    {
        return FuelAfterLowerThanFuelSnafu { fuel_after, fuel }.fail();
    };

    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    db.update_fuel_measurements(&body, call_sign, user_id)
        .await?;

    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn delete_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<Vec<DeleteFuelMeasurement>>,
) -> Result<Response<()>> {
    let call_sign = profile.call_sign()?;

    db.delete_fuel_measurements(&body.into_inner(), call_sign)
        .await?;
    Ok(Response::new(()))
}

impl FuelMeasurementsParams {
    pub fn to_query(self, call_sign: CallSign) -> FuelMeasurementsQuery {
        let Self {
            start_date,
            end_date,
        } = self;

        FuelMeasurementsQuery {
            call_sign,
            start_date,
            end_date,
        }
    }
}
