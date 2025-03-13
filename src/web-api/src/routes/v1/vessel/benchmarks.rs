use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web::{self};
use kyogre_core::VesselBenchmarks;
use oasgen::oasgen;

/// Returns benchmark data for the vessel associated with the authenticated user.
#[oasgen(skip(db), tags("Vessel"))]
#[tracing::instrument(skip(db), fields(user_id = ?profile.id()))]
pub async fn benchmarks<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<VesselBenchmarks>> {
    let call_sign = profile.call_sign()?;
    Ok(Response::new(
        db.vessel_benchmarks(&profile.user.id, call_sign).await?,
    ))
}
