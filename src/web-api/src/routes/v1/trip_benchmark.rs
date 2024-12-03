use crate::{
    error::{ErrorResponse, Result},
    extractors::BwProfile,
    response::Response,
    Database,
};
use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, GearGroup, VesselLengthGroup};
use kyogre_core::{
    AverageEeoiQuery, AverageTripBenchmarks, AverageTripBenchmarksQuery, EeoiQuery,
    FiskeridirVesselId, Mean, Ordering, TripBenchmarksQuery, TripId, TripWithBenchmark,
};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use utoipa::{IntoParams, ToSchema};

#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmarksParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub ordering: Option<Ordering>,
}

#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct EeoiParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AverageTripBenchmarksParams {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    #[serde(default)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    #[param(rename = "gearGroups[]", value_type = Option<Vec<GearGroup>>)]
    pub gear_groups: Vec<GearGroup>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub length_group: Option<VesselLengthGroup>,
    #[serde(default)]
    #[param(rename = "vesselIds[]", value_type = Option<Vec<i64>>)]
    pub vessel_ids: Vec<FiskeridirVesselId>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AverageEeoiParams {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    #[serde(default)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    #[param(rename = "gearGroups[]", value_type = Option<Vec<GearGroup>>)]
    pub gear_groups: Vec<GearGroup>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub length_group: Option<VesselLengthGroup>,
    #[serde(default)]
    #[param(rename = "vesselIds[]", value_type = Option<Vec<i64>>)]
    pub vessel_ids: Vec<FiskeridirVesselId>,
}

#[utoipa::path(
    get,
    path = "/trip_benchmarks/average",
    params(AverageTripBenchmarksParams),
    responses(
        (status = 200, description = "average trip benchmarks", body = AverageTripBenchmarks),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn average<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<AverageTripBenchmarksParams>,
) -> Result<Response<AverageTripBenchmarks>> {
    let query = params.into_inner().into();
    Ok(Response::new(db.average_trip_benchmarks(query).await?))
}

#[utoipa::path(
    get,
    path = "/trip_benchmarks",
    params(TripBenchmarksParams),
    responses(
        (status = 200, description = "your trip benchmarks matching the parameters", body = TripBenchmarks),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn trip_benchmarks<T: Database>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<TripBenchmarksParams>,
) -> Result<Response<TripBenchmarks>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().into_query(call_sign.clone());

    let benchmarks = db.trip_benchmarks(query).await?.into();
    Ok(Response::new(benchmarks))
}

/// Returns the EEOI of the logged in user for the given period.
/// EEOI is given with the unit: `tonn / (tonn * nautical miles)`
#[utoipa::path(
    get,
    path = "/trip_benchmarks/eeoi",
    params(EeoiParams),
    responses(
        (status = 200, description = "your EEOI for the given period", body = Option<f64>),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn eeoi<T: Database>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<EeoiParams>,
) -> Result<Response<Option<f64>>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().into_query(call_sign.clone());

    let eeoi = db.eeoi(query).await?;
    Ok(Response::new(eeoi))
}

/// Returns the average EEOI of all vessels matching the given parameters.
/// EEOI is given with the unit: `tonn / (tonn * nautical miles)`
#[utoipa::path(
    get,
    path = "/trip_benchmarks/average_eeoi",
    params(AverageEeoiParams),
    responses(
        (status = 200, description = "the average EEOI for vessels matching the parameters", body = Option<f64>),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn average_eeoi<T: Database>(
    db: web::Data<T>,
    params: Query<AverageEeoiParams>,
) -> Result<Response<Option<f64>>> {
    let query = params.into_inner().into();
    let eeoi = db.average_eeoi(query).await?;
    Ok(Response::new(eeoi))
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
    pub catch_value_per_fuel: Option<f64>,
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
    pub catch_value_per_fuel: Option<f64>,
    pub eeoi: Option<f64>,
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
            catch_value_per_fuel: v.iter().filter_map(|v| v.catch_value_per_fuel).mean(),
            trips: v.into_iter().map(From::from).collect(),
        }
    }
}

impl From<TripWithBenchmark> for TripBenchmark {
    fn from(v: TripWithBenchmark) -> Self {
        let TripWithBenchmark {
            id,
            period,
            period_precision,
            weight_per_hour,
            weight_per_distance,
            weight_per_fuel,
            catch_value_per_fuel,
            fuel_consumption,
            eeoi,
        } = v;

        let period = period_precision.unwrap_or(period);

        Self {
            id,
            start: period.start(),
            end: period.end(),
            weight_per_hour,
            weight_per_distance,
            fuel_consumption,
            weight_per_fuel,
            catch_value_per_fuel,
            eeoi,
        }
    }
}

impl TripBenchmarksParams {
    fn into_query(self, call_sign: CallSign) -> TripBenchmarksQuery {
        let Self {
            start_date,
            end_date,
            ordering,
        } = self;

        TripBenchmarksQuery {
            call_sign,
            start_date,
            end_date,
            ordering: ordering.unwrap_or_default(),
        }
    }
}

impl EeoiParams {
    fn into_query(self, call_sign: CallSign) -> EeoiQuery {
        let Self {
            start_date,
            end_date,
        } = self;

        EeoiQuery {
            call_sign,
            start_date,
            end_date,
        }
    }
}

impl From<AverageTripBenchmarksParams> for AverageTripBenchmarksQuery {
    fn from(v: AverageTripBenchmarksParams) -> Self {
        let AverageTripBenchmarksParams {
            start_date,
            end_date,
            gear_groups,
            length_group,
            vessel_ids,
        } = v;

        Self {
            start_date,
            end_date,
            gear_groups,
            length_group,
            vessel_ids,
        }
    }
}

impl From<AverageEeoiParams> for AverageEeoiQuery {
    fn from(v: AverageEeoiParams) -> Self {
        let AverageEeoiParams {
            start_date,
            end_date,
            gear_groups,
            length_group,
            vessel_ids,
        } = v;

        Self {
            start_date,
            end_date,
            gear_groups,
            length_group,
            vessel_ids,
        }
    }
}
