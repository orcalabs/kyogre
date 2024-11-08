use crate::{
    error::{ErrorResponse, Result},
    extractors::BwProfile,
    response::Response,
    Database,
};
use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, GearGroup, VesselLengthGroup};
use kyogre_core::{Mean, Ordering, TripBenchmarksQuery, TripId, TripWithBenchmark};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmarksParameters {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub ordering: Option<Ordering>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FuelConsumptionAverageParams {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    #[serde(default)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    #[param(rename = "gearGroups[]", value_type = Option<Vec<GearGroup>>)]
    pub gear_groups: Vec<GearGroup>,
    pub length_group: Option<VesselLengthGroup>,
}

#[utoipa::path(
    get,
    path = "/trip_benchmarks/average_fuel_consumption",
    params(FuelConsumptionAverageParams),
    responses(
        (status = 200, description = "average fuel consumption", body = Option<f64>),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn average_fuel_consumption<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<FuelConsumptionAverageParams>,
) -> Result<Response<Option<f64>>> {
    let params = params.into_inner();
    Ok(Response::new(
        db.average_fuel_consumption(
            params.start_date,
            params.end_date,
            params.gear_groups,
            params.length_group,
        )
        .await?,
    ))
}

#[utoipa::path(
    get,
    path = "/trip_benchmarks",
    params(TripBenchmarksParameters),
    responses(
        (status = 200, description = "your trip benchmarks matching the parameters", body = TripBenchmarks),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_benchmarks<T: Database>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<TripBenchmarksParameters>,
) -> Result<Response<TripBenchmarks>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().to_query(call_sign);

    let benchmarks = db.trip_benchmarks(query).await?.into();
    Ok(Response::new(benchmarks))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmarks {
    // TODO
    // pub total_sustainability: f64,
    pub weight_per_hour: Option<f64>,
    pub weight_per_distance: Option<f64>,
    pub fuel_consumption: Option<f64>,
    pub weight_per_fuel: Option<f64>,
    pub trips: Vec<TripBenchmark>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmark {
    #[schema(value_type = i64)]
    pub id: TripId,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub weight_per_hour: Option<f64>,
    pub weight_per_distance: Option<f64>,
    pub fuel_consumption: Option<f64>,
    pub weight_per_fuel: Option<f64>,
    // TODO
    // pub sustainability: f64,
}

impl From<Vec<TripWithBenchmark>> for TripBenchmarks {
    fn from(v: Vec<TripWithBenchmark>) -> Self {
        Self {
            weight_per_hour: v.iter().filter_map(|v| v.weight_per_hour).mean(),
            weight_per_distance: v.iter().filter_map(|v| v.weight_per_distance).mean(),
            fuel_consumption: v
                .iter()
                .filter_map(|v| v.fuel_consumption)
                .fold(None, |acc, cur| Some(acc.unwrap_or(0.) + cur)),
            weight_per_fuel: v.iter().filter_map(|v| v.weight_per_fuel).mean(),
            trips: v.into_iter().map(From::from).collect(),
        }
    }
}

impl From<TripWithBenchmark> for TripBenchmark {
    fn from(v: TripWithBenchmark) -> Self {
        let period = v.period_precision.unwrap_or(v.period);
        Self {
            id: v.id,
            start: period.start(),
            end: period.end(),
            weight_per_hour: v.weight_per_hour,
            weight_per_distance: v.weight_per_distance,
            fuel_consumption: v.fuel_consumption,
            weight_per_fuel: v.weight_per_fuel,
        }
    }
}

impl TripBenchmarksParameters {
    fn to_query(&self, call_sign: CallSign) -> TripBenchmarksQuery {
        TripBenchmarksQuery {
            call_sign,
            start_date: self.start_date,
            end_date: self.end_date,
            ordering: self.ordering.unwrap_or_default(),
        }
    }
}
