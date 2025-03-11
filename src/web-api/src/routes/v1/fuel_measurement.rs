use actix_web::web;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc, offset::LocalResult};
use chrono_tz::Europe::Oslo;
use fiskeridir_rs::CallSign;
use kyogre_core::{
    CreateFuelMeasurement, DeleteFuelMeasurement, FuelMeasurement, FuelMeasurementsQuery,
};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Deserializer, Serialize, de::Unexpected};
use serde_qs::actix::QsQuery as Query;

use crate::{
    Database,
    error::{Result, error::FuelAfterLowerThanFuelSnafu},
    excel::decode_excel_base64,
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response,
};

#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurementsParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize, OaSchema)]
pub struct UploadFuelMeasurement {
    pub file: String,
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
    if let Some((fuel_after_liter, fuel_liter)) = body
        .iter()
        .filter_map(|b| b.fuel_after_liter.map(|a| (a, b.fuel_liter)))
        .find(|v| v.0 <= v.1)
    {
        return FuelAfterLowerThanFuelSnafu {
            fuel_after_liter,
            fuel_liter,
        }
        .fail();
    };

    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    let measurements = db.add_fuel_measurements(&body, call_sign, user_id).await?;

    Ok(Response::new(measurements))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db, body))]
pub async fn upload_fuel_measurements<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<UploadFuelMeasurement>,
) -> Result<Response<Vec<FuelMeasurement>>> {
    let user_id = profile.user.id;
    let call_sign = profile.call_sign()?;

    #[derive(Deserialize)]
    struct Record {
        #[serde(deserialize_with = "deserialize_norwegian_timestamp")]
        pub timestamp: DateTime<Utc>,
        pub fuel_liter_before: f64,
        #[serde(default)]
        pub fuel_after_liter: Option<f64>,
    }

    let measurements = decode_excel_base64(body.into_inner().file)?
        .into_iter()
        .map(|v: Record| CreateFuelMeasurement {
            timestamp: v.timestamp,
            fuel_liter: v.fuel_liter_before,
            fuel_after_liter: v.fuel_after_liter,
        })
        .collect::<Vec<_>>();

    let measurements = db
        .add_fuel_measurements(&measurements, call_sign, user_id)
        .await?;

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
    if let Some((fuel_after_liter, fuel_liter)) = body
        .iter()
        .filter_map(|b| b.fuel_after_liter.map(|a| (a, b.fuel_liter)))
        .find(|v| v.0 <= v.1)
    {
        return FuelAfterLowerThanFuelSnafu {
            fuel_after_liter,
            fuel_liter,
        }
        .fail();
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

pub fn deserialize_norwegian_timestamp<'de, D>(
    deserializer: D,
) -> std::result::Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    let err = || {
        Err(serde::de::Error::invalid_value(
            Unexpected::Str(&s),
            &"a valid date-time with with format: 'dd.mm.yyyy HH:MM:SS'",
        ))
    };

    match NaiveDateTime::parse_from_str(&s, "%d.%m.%Y %H:%M:%S") {
        Ok(v) => {
            let dt = match Oslo.from_local_datetime(&v) {
                LocalResult::Single(v) => v,
                // As we have no way of knowing if the timestamp is before or after winter/summer
                // time shift we simply have to pick one.
                LocalResult::Ambiguous(_, v) => v,
                LocalResult::None => {
                    return err();
                }
            };

            Ok(dt.with_timezone(&Utc))
        }
        Err(_) => err(),
    }
}
