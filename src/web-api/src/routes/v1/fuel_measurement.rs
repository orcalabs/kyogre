use crate::{
    Database,
    error::Result,
    extractors::BwProfile,
    response::{Response, StreamResponse},
    stream_response,
};
use actix_web::web;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc, offset::LocalResult};
use chrono_tz::Europe::Oslo;
use fiskeridir_rs::CallSign;
use kyogre_core::{
    CreateFuelMeasurement, DeleteFuelMeasurement, FuelMeasurement, FuelMeasurementsQuery,
    OptionalDateTimeRange,
};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Deserializer, Serialize, de::Unexpected};
use serde_qs::actix::QsQuery as Query;

#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurementsParams {
    #[serde(flatten)]
    pub range: OptionalDateTimeRange,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn get_fuel_measurements<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FuelMeasurementsParams>,
) -> Result<StreamResponse<FuelMeasurement>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let query = params.into_inner().to_query(call_sign.clone());

    let response = stream_response! {
        db.fuel_measurements(query)
    };

    Ok(response)
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn create_fuel_measurement<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<CreateFuelMeasurement>,
) -> Result<Response<FuelMeasurement>> {
    let body = body.into_inner();

    let user_id = profile.user.id;
    let call_sign = profile.call_sign(db.as_ref()).await?;

    let measurement = db.add_fuel_measurement(&body, &call_sign, user_id).await?;

    Ok(Response::new(measurement))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn update_fuel_measurement<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<FuelMeasurement>,
) -> Result<Response<()>> {
    let body = body.into_inner();

    let user_id = profile.user.id;
    let call_sign = profile.call_sign(db.as_ref()).await?;

    db.update_fuel_measurement(&body, &call_sign, user_id)
        .await?;

    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("FuelMeasurement"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn delete_fuel_measurement<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<DeleteFuelMeasurement>,
) -> Result<Response<()>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;

    db.delete_fuel_measurement(&body.into_inner(), &call_sign)
        .await?;
    Ok(Response::new(()))
}

impl FuelMeasurementsParams {
    pub fn to_query(self, call_sign: CallSign) -> FuelMeasurementsQuery {
        let Self {
            range,
            limit,
            offset,
        } = self;

        FuelMeasurementsQuery {
            call_sign,
            range,
            limit,
            offset,
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
