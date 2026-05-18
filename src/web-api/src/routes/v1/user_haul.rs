use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web::{self, Path};
use kyogre_core::{HaulEnd, HaulStart, StartedUserHaul, UpdateUserHaul, UserHaul, UserHaulId};
use oasgen::oasgen;

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn user_hauls<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<Vec<UserHaul>>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let hauls = db.user_hauls(&call_sign).await?;

    Ok(Response::new(hauls))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn start_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    start: web::Json<HaulStart>,
) -> Result<Response<StartedUserHaul>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let haul = db
        .start_user_haul(&call_sign, profile.user.id, &start)
        .await?;
    Ok(Response::new(haul))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn abort_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<()>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    db.abort_user_haul(&call_sign).await?;
    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn update_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    path: Path<UserHaulId>,
    update: web::Json<UpdateUserHaul>,
) -> Result<Response<UserHaul>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let user_haul = db
        .update_user_haul(&call_sign, path.into_inner(), &update)
        .await?;
    Ok(Response::new(user_haul))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn update_current_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    update: web::Json<HaulStart>,
) -> Result<Response<StartedUserHaul>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let user_haul = db.update_current_user_haul(&call_sign, &update).await?;
    Ok(Response::new(user_haul))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn stop_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    end: web::Json<HaulEnd>,
) -> Result<Response<UserHaul>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let user_haul = db.stop_user_haul(&call_sign, &end, profile.user.id).await?;
    Ok(Response::new(user_haul))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn delete_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    path: web::Path<UserHaulId>,
) -> Result<Response<()>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    db.delete_user_haul(&call_sign, path.into_inner()).await?;
    Ok(Response::new(()))
}

#[oasgen(skip(db), tags("UserHaul"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn current_user_haul<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<Option<StartedUserHaul>>> {
    let call_sign = profile.call_sign(db.as_ref()).await?;
    let current = db.current_user_haul(&call_sign).await?;
    Ok(Response::new(current))
}
