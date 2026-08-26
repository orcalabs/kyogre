use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web;
use kyogre_core::{Bunkering, BunkeringId, CreateBunkering};
use oasgen::oasgen;

#[oasgen(skip(db), tags("Bunkering"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn create_bunkering<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    body: web::Json<CreateBunkering>,
) -> Result<Response<Bunkering>> {
    let body = body.into_inner();

    let user_id = profile.user.id;
    let call_sign = profile.call_sign(db.as_ref()).await?;

    let measurement = db.add_bunkering(&body, &call_sign, user_id).await?;

    Ok(Response::new(measurement))
}

#[oasgen(skip(db), tags("Bunkering"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn update_bunkering<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    path: web::Path<BunkeringId>,
    body: web::Json<CreateBunkering>,
) -> Result<Response<()>> {
    let body = body.into_inner();

    let user_id = profile.user.id;
    let call_sign = profile.call_sign(db.as_ref()).await?;

    db.update_bunkering(path.into_inner(), &body, &call_sign, user_id)
        .await?;

    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("Bunkering"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn delete_bunkering<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    path: web::Path<BunkeringId>,
) -> Result<Response<()>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;

    db.delete_bunkering(path.into_inner(), &call_sign).await?;
    Ok(Response::new(()))
}
