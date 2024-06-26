use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{BarentswatchUserId, FuelMeasurementsQuery};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::ApiError, extractors::BwProfile, response::Response, to_streaming_response, Database,
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
pub async fn get_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FuelMeasurementsParams>,
) -> Result<HttpResponse, ApiError> {
    let user_id = BarentswatchUserId(profile.user.id);

    let profile = profile
        .fisk_info_profile
        .ok_or(ApiError::MissingBwFiskInfoProfile)?;
    let call_sign = CallSign::try_from(profile.ircs).map_err(|_| ApiError::InvalidCallSign)?;

    let query = params.into_inner().to_query(user_id, call_sign);

    to_streaming_response! {
        db.fuel_measurements(query)
            .map_ok(FuelMeasurement::from)
            .map_err(|e| {
                event!(Level::ERROR, "failed to get fuel measurements: {:?}", e);
                ApiError::InternalServerError
            })
    }
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
) -> Result<Response<()>, ApiError> {
    let user_id = BarentswatchUserId(profile.user.id);

    let profile = profile
        .fisk_info_profile
        .ok_or(ApiError::MissingBwFiskInfoProfile)?;
    let call_sign = CallSign::try_from(profile.ircs).map_err(|_| ApiError::InvalidCallSign)?;

    let measurements = body
        .into_inner()
        .into_iter()
        .map(|m| m.to_domain_fuel_measurement(user_id, &call_sign))
        .collect();

    db.add_fuel_measurements(measurements)
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to create fuel measurements: {:?}", e);
            ApiError::InternalServerError
        })
        .map(|_| Response::new(()))
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
) -> Result<Response<()>, ApiError> {
    let user_id = BarentswatchUserId(profile.user.id);

    let profile = profile
        .fisk_info_profile
        .ok_or(ApiError::MissingBwFiskInfoProfile)?;
    let call_sign = CallSign::try_from(profile.ircs).map_err(|_| ApiError::InvalidCallSign)?;

    let measurements = body
        .into_inner()
        .into_iter()
        .map(|m| m.to_domain_fuel_measurement(user_id, &call_sign))
        .collect();

    db.update_fuel_measurements(measurements)
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to update fuel measurements: {:?}", e);
            ApiError::InternalServerError
        })
        .map(|_| Response::new(()))
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
) -> Result<Response<()>, ApiError> {
    let user_id = BarentswatchUserId(profile.user.id);

    let profile = profile
        .fisk_info_profile
        .ok_or(ApiError::MissingBwFiskInfoProfile)?;
    let call_sign = CallSign::try_from(profile.ircs).map_err(|_| ApiError::InvalidCallSign)?;

    let measurements = body
        .into_inner()
        .into_iter()
        .map(|m| m.to_domain_delete_fuel_measurement(user_id, &call_sign))
        .collect();

    db.delete_fuel_measurements(measurements)
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to delete fuel measurements: {:?}", e);
            ApiError::InternalServerError
        })
        .map(|_| Response::new(()))
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
