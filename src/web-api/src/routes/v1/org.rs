use super::vessel::FuelParams;
use crate::error::error::{MissingDateRangeSnafu, OrgNotFoundSnafu};
use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web::{self, Path};
use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::{CallSign, OrgId};
use kyogre_core::{FuelEntry, OrgBenchmarkQuery, OrgBenchmarks};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;

#[derive(Default, Debug, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrgBenchmarkParameters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, OaSchema, Deserialize)]
pub struct OrgBenchmarkPath {
    pub org_id: OrgId,
}
#[derive(Debug, Clone, OaSchema, Deserialize)]
pub struct OrgFuelPath {
    pub org_id: OrgId,
}

/// Returns organization benchmarks for the given organization id (Breg org id).
/// This will include benchmarks for all vessels associated with the organization.
#[oasgen(skip(db), tags("Org"))]
#[tracing::instrument(skip(db))]
pub async fn benchmarks<T: Database + 'static>(
    db: web::Data<T>,
    bw_profile: BwProfile,
    params: Query<OrgBenchmarkParameters>,
    path: Path<OrgBenchmarkPath>,
) -> Result<Response<OrgBenchmarks>> {
    let call_sign = bw_profile.call_sign()?;
    let query = params
        .into_inner()
        .into_query(call_sign.clone(), path.org_id)?;

    match db.org_benchmarks(&query).await? {
        Some(b) => Ok(Response::new(b)),
        None => OrgNotFoundSnafu {
            org_id: path.org_id,
        }
        .fail(),
    }
}

/// Returns a fuel consumption estimate for the given date range for all vessels associated with the
/// given org, if no date range is given the last 30 days
/// are returned.
/// This is not based on trips and is the full fuel consumption estimate for the given date range.
#[oasgen(skip(db), tags("Org"))]
#[tracing::instrument(skip(db))]
pub async fn fuel<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    params: Query<FuelParams>,
    path: Path<OrgFuelPath>,
) -> Result<Response<Vec<FuelEntry>>> {
    let call_sign = profile.call_sign()?;
    let query = params.into_inner().to_query(call_sign.clone())?;

    let org_id = path.into_inner().org_id;
    match db.fuel_estimation_by_org(&query, org_id).await? {
        Some(b) => Ok(Response::new(b)),
        None => OrgNotFoundSnafu { org_id }.fail(),
    }
}

impl OrgBenchmarkParameters {
    pub fn into_query(self, call_sign: CallSign, org_id: OrgId) -> Result<OrgBenchmarkQuery> {
        let (start, end) = match (self.start, self.end) {
            (None, None) => {
                let now = Utc::now();
                Ok((now - Duration::days(30), now))
            }
            (Some(s), Some(e)) => Ok((s, e)),
            _ => MissingDateRangeSnafu {
                start: self.start.is_some(),
                end: self.end.is_some(),
            }
            .fail(),
        }?;
        Ok(OrgBenchmarkQuery {
            start,
            end,
            call_sign,
            org_id,
        })
    }
}
