use crate::{
    error::Result,
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response, Database,
};
use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{BarentswatchUserId, FuelMeasurementsQuery};
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
    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().to_query(user_id, call_sign.clone());

    let response = stream_response! {
        db.fuel_measurements(query).map_ok(FuelMeasurement::from)
    };

    Ok(response)
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn create_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<Vec<FuelMeasurementBody>>,
) -> Result<Response<()>> {
    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    let measurements: Vec<kyogre_core::FuelMeasurement> = body
        .into_inner()
        .into_iter()
        .map(|m| m.to_domain_fuel_measurement(user_id, call_sign))
        .collect();

    db.add_fuel_measurements(&measurements).await?;
    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn update_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<Vec<FuelMeasurementBody>>,
) -> Result<Response<()>> {
    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    let measurements: Vec<kyogre_core::FuelMeasurement> = body
        .into_inner()
        .into_iter()
        .map(|m| m.to_domain_fuel_measurement(user_id, call_sign))
        .collect();

    db.update_fuel_measurements(&measurements).await?;
    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db))]
pub async fn delete_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<Vec<DeleteFuelMeasurement>>,
) -> Result<Response<()>> {
    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    let measurements: Vec<kyogre_core::DeleteFuelMeasurement> = body
        .into_inner()
        .into_iter()
        .map(|m| m.to_domain_delete_fuel_measurement(user_id, call_sign))
        .collect();

    db.delete_fuel_measurements(&measurements).await?;
    Ok(Response::new(()))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurement {
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurementBody {
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFuelMeasurement {
    pub timestamp: DateTime<Utc>,
}

impl From<kyogre_core::FuelMeasurement> for FuelMeasurement {
    fn from(v: kyogre_core::FuelMeasurement) -> Self {
        let kyogre_core::FuelMeasurement {
            barentswatch_user_id,
            call_sign,
            timestamp,
            fuel,
        } = v;

        Self {
            barentswatch_user_id,
            call_sign,
            timestamp,
            fuel,
        }
    }
}

impl FuelMeasurementBody {
    pub fn to_domain_fuel_measurement(
        self,
        barentswatch_user_id: BarentswatchUserId,
        call_sign: &CallSign,
    ) -> kyogre_core::FuelMeasurement {
        let Self { timestamp, fuel } = self;

        kyogre_core::FuelMeasurement {
            barentswatch_user_id,
            call_sign: call_sign.clone(),
            timestamp,
            fuel,
        }
    }
}

impl FuelMeasurementsParams {
    pub fn to_query(
        self,
        barentswatch_user_id: BarentswatchUserId,
        call_sign: CallSign,
    ) -> FuelMeasurementsQuery {
        let Self {
            start_date,
            end_date,
        } = self;

        FuelMeasurementsQuery {
            barentswatch_user_id,
            call_sign,
            start_date,
            end_date,
        }
    }
}

impl DeleteFuelMeasurement {
    pub fn to_domain_delete_fuel_measurement(
        self,
        barentswatch_user_id: BarentswatchUserId,
        call_sign: &CallSign,
    ) -> kyogre_core::DeleteFuelMeasurement {
        let Self { timestamp } = self;

        kyogre_core::DeleteFuelMeasurement {
            barentswatch_user_id,
            call_sign: call_sign.clone(),
            timestamp,
        }
    }
}
