use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{BarentswatchUserId, FuelMeasurementsQuery};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::{ErrorResponse, Result},
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response, Database,
};

#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurementsParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/fuel_measurements",
    params(FuelMeasurementsParams),
    responses(
        (status = 200, description = "fuel measurements", body = [FuelMeasurement]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    post,
    path = "/fuel_measurements",
    request_body(
        content = [FuelMeasurementBody],
        content_type = "application/json",
        description = "fuel measurements",
    ),
    responses(
        (status = 200, description = "create successfull"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    put,
    path = "/fuel_measurements",
    request_body(
        content = [FuelMeasurementBody],
        content_type = "application/json",
        description = "updated fuel measurements",
    ),
    responses(
        (status = 200, description = "update successfull"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    delete,
    path = "/fuel_measurements",
    request_body(
        content = [DeleteFuelMeasurement],
        content_type = "application/json",
        description = "fuel measurements to delete",
    ),
    responses(
        (status = 200, description = "delete successfull"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurement {
    #[schema(value_type = Uuid)]
    pub barentswatch_user_id: BarentswatchUserId,
    #[schema(value_type = String)]
    pub call_sign: CallSign,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurementBody {
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFuelMeasurement {
    pub timestamp: DateTime<Utc>,
}

impl From<kyogre_core::FuelMeasurement> for FuelMeasurement {
    fn from(v: kyogre_core::FuelMeasurement) -> Self {
        Self {
            barentswatch_user_id: v.barentswatch_user_id,
            call_sign: v.call_sign,
            timestamp: v.timestamp,
            fuel: v.fuel,
        }
    }
}

impl FuelMeasurementBody {
    pub fn to_domain_fuel_measurement(
        self,
        barentswatch_user_id: BarentswatchUserId,
        call_sign: &CallSign,
    ) -> kyogre_core::FuelMeasurement {
        kyogre_core::FuelMeasurement {
            barentswatch_user_id,
            call_sign: call_sign.clone(),
            timestamp: self.timestamp,
            fuel: self.fuel,
        }
    }
}

impl FuelMeasurementsParams {
    pub fn to_query(
        self,
        barentswatch_user_id: BarentswatchUserId,
        call_sign: CallSign,
    ) -> FuelMeasurementsQuery {
        FuelMeasurementsQuery {
            barentswatch_user_id,
            call_sign,
            start_date: self.start_date,
            end_date: self.end_date,
        }
    }
}

impl DeleteFuelMeasurement {
    pub fn to_domain_delete_fuel_measurement(
        self,
        barentswatch_user_id: BarentswatchUserId,
        call_sign: &CallSign,
    ) -> kyogre_core::DeleteFuelMeasurement {
        kyogre_core::DeleteFuelMeasurement {
            barentswatch_user_id,
            call_sign: call_sign.clone(),
            timestamp: self.timestamp,
        }
    }
}
