use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web;
use chrono::{DateTime, Utc};
use fiskeridir_rs::SpeciesGroup;
use fiskeridir_rs::{CallSign, GearGroup, VesselLengthGroup};
use kyogre_core::{
    AverageEeoiQuery, AverageFuiQuery, AverageTripBenchmarks, AverageTripBenchmarksQuery,
    DateTimeRange, EeoiQuery, FiskeridirVesselId, FuiQuery, Mean, OptionalDateTimeRange, Ordering,
    TripBenchmarksQuery, TripId, TripWithBenchmark,
};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::DisplayFromStr;
use serde_with::serde_as;

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct TripBenchmarksParams {
    #[serde(flatten)]
    pub range: OptionalDateTimeRange,
    pub ordering: Option<Ordering>,
}

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct EeoiParams {
    #[serde(flatten)]
    pub range: OptionalDateTimeRange,
}

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct FuiParams {
    #[serde(flatten)]
    pub range: OptionalDateTimeRange,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AverageTripBenchmarksParams {
    #[serde(flatten)]
    pub range: DateTimeRange,
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
pub struct AverageFuiParams {
    #[serde(flatten)]
    pub range: DateTimeRange,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    #[oasgen(rename = "gearGroups[]")]
    pub gear_groups: Option<Vec<GearGroup>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub length_group: Option<VesselLengthGroup>,
    #[oasgen(rename = "vesselIds[]")]
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub species_group_id: Option<SpeciesGroup>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct AverageEeoiParams {
    #[serde(flatten)]
    pub range: DateTimeRange,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    #[oasgen(rename = "gearGroups[]")]
    pub gear_groups: Option<Vec<GearGroup>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub length_group: Option<VesselLengthGroup>,
    #[oasgen(rename = "vesselIds[]")]
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub species_group_id: Option<SpeciesGroup>,
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

/// Returns the FUI of the logged in user for the given period.
#[oasgen(skip(db), tags("Trip"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn fui<T: Database>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FuiParams>,
) -> Result<Response<Option<f64>>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().into_query(call_sign.clone());

    let fui = db.fui(query).await?;
    Ok(Response::new(fui))
}

/// Returns the average FUI of all vessels matching the given parameters.
#[oasgen(skip(db), tags("Trip"))]
#[tracing::instrument(skip(db))]
pub async fn average_fui<T: Database>(
    db: web::Data<T>,
    params: Query<AverageFuiParams>,
) -> Result<Response<Option<f64>>> {
    let query = params.into_inner().into();
    let fui = db.average_fui(query).await?;
    Ok(Response::new(fui))
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
        let Self { ordering, range } = self;

        TripBenchmarksQuery {
            call_sign,
            range,
            ordering: ordering.unwrap_or_default(),
        }
    }
}

impl FuiParams {
    fn into_query(self, call_sign: CallSign) -> FuiQuery {
        let Self { range } = self;

        FuiQuery { call_sign, range }
    }
}

impl EeoiParams {
    fn into_query(self, call_sign: CallSign) -> EeoiQuery {
        let Self { range } = self;

        EeoiQuery { call_sign, range }
    }
}

impl From<AverageTripBenchmarksParams> for AverageTripBenchmarksQuery {
    fn from(v: AverageTripBenchmarksParams) -> Self {
        let AverageTripBenchmarksParams {
            range,
            gear_groups,
            length_group,
            vessel_ids,
        } = v;

        Self {
            range,
            gear_groups: gear_groups.unwrap_or_default(),
            length_group,
            vessel_ids: vessel_ids.unwrap_or_default(),
        }
    }
}

impl From<AverageEeoiParams> for AverageEeoiQuery {
    fn from(v: AverageEeoiParams) -> Self {
        let AverageEeoiParams {
            range,
            gear_groups,
            length_group,
            vessel_ids,
            species_group_id,
        } = v;

        Self {
            range,
            gear_groups: gear_groups.unwrap_or_default(),
            length_group,
            vessel_ids: vessel_ids.unwrap_or_default(),
            species_group_id,
        }
    }
}

impl From<AverageFuiParams> for AverageFuiQuery {
    fn from(v: AverageFuiParams) -> Self {
        let AverageFuiParams {
            range,
            gear_groups,
            length_group,
            vessel_ids,
            species_group_id,
        } = v;

        Self {
            range,
            gear_groups: gear_groups.unwrap_or_default(),
            length_group,
            vessel_ids: vessel_ids.unwrap_or_default(),
            species_group_id,
        }
    }
}
