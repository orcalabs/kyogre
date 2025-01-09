use crate::{error::Result, extractors::BwProfile, response::Response, Database};
use actix_web::web::{self};
use kyogre_core::VesselBenchmarks;
use oasgen::oasgen;

/// Returns benchmark data for the vessel associated with the authenticated user.
#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db))]
pub async fn benchmarks<T: Database + 'static>(
    db: web::Data<T>,
    bw_profile: BwProfile,
) -> Result<Response<VesselBenchmarks>> {
    let call_sign = bw_profile.call_sign()?;
    Ok(Response::new(
        db.vessel_benchmarks(&bw_profile.user.id, call_sign).await?,
    ))
}
