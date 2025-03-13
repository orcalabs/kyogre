use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, GearGroup, VesselLengthGroup};
use kyogre_core::{
    AverageEeoiQuery, AverageTripBenchmarks, AverageTripBenchmarksQuery, EeoiQuery,
    FiskeridirVesselId, Mean, Ordering, TripBenchmarksQuery, TripId, TripWithBenchmark,
};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::DisplayFromStr;
use serde_with::serde_as;

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmarksParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub ordering: Option<Ordering>,
}

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct EeoiParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AverageTripBenchmarksParams {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    #[oasgen(rename = "gearGroups[]")]
    pub gear_groups: Option<Vec<GearGroup>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub length_group: Option<VesselLengthGroup>,
    #[oasgen(rename = "vesselIds[]")]
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AverageEeoiParams {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    #[oasgen(rename = "gearGroups[]")]
    pub gear_groups: Option<Vec<GearGroup>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub length_group: Option<VesselLengthGroup>,
    #[oasgen(rename = "vesselIds[]")]
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

/// Returns the average trip benchmarks for the given timespan and vessels matching the given
/// parameters.
#[oasgen(skip(db), tags("Trip"))]
#[tracing::instrument(skip(db))]
pub async fn average<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    params: Query<AverageTripBenchmarksParams>,
) -> Result<Response<AverageTripBenchmarks>> {
    let query = params.into_inner().into();
    Ok(Response::new(db.average_trip_benchmarks(query).await?))
}

/// Returns trip benchmarks for the vessel associated with the authenticated user.
#[oasgen(skip(db), tags("Trip"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn benchmarks<T: Database>(
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
#[oasgen(skip(db), tags("Trip"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
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
#[oasgen(skip(db), tags("Trip"))]
#[tracing::instrument(skip(db))]
pub async fn average_eeoi<T: Database>(
    db: web::Data<T>,
    params: Query<AverageEeoiParams>,
) -> Result<Response<Option<f64>>> {
    let query = params.into_inner().into();
    let eeoi = db.average_eeoi(query).await?;
    Ok(Response::new(eeoi))
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmark {
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
                .filter_map(|v| v.fuel_consumption_liter)
                .fold(None, |acc, cur| Some(acc.unwrap_or(0.) + cur)),
            weight_per_fuel: v.iter().filter_map(|v| v.weight_per_fuel_liter).mean(),
            catch_value_per_fuel: v.iter().filter_map(|v| v.catch_value_per_fuel_liter).mean(),
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
            weight_per_fuel_liter,
            catch_value_per_fuel_liter,
            fuel_consumption_liter,
            eeoi,
        } = v;

        let period = period_precision.unwrap_or(period);

        Self {
            id,
            start: period.start(),
            end: period.end(),
            weight_per_hour,
            weight_per_distance,
            fuel_consumption: fuel_consumption_liter,
            weight_per_fuel: weight_per_fuel_liter,
            catch_value_per_fuel: catch_value_per_fuel_liter,
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
            gear_groups: gear_groups.unwrap_or_default(),
            length_group,
            vessel_ids: vessel_ids.unwrap_or_default(),
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
            gear_groups: gear_groups.unwrap_or_default(),
            length_group,
            vessel_ids: vessel_ids.unwrap_or_default(),
        }
    }
}
