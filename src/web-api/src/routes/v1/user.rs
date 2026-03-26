use crate::{Database, error::Result, extractors::BwProfile, response::Response};
use actix_web::web;
use kyogre_core::{FiskeridirVesselId, UpdateUser};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};

#[oasgen(skip(db), tags("User"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn get_user<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<User>> {
    Ok(Response::new(db.get_user(profile.user.id).await?.into()))
}

#[oasgen(skip(db), tags("User"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn update_user<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    user: web::Json<UpdateUser>,
) -> Result<Response<()>> {
    db.update_user(&user, profile.user.id).await?;
    Ok(Response::new(()))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub following: Vec<FiskeridirVesselId>,
    pub fuel_consent: Option<bool>,
}

impl From<kyogre_core::User> for User {
    fn from(v: kyogre_core::User) -> Self {
        let kyogre_core::User {
            barentswatch_user_id: _,
            following,
            fuel_consent,
        } = v;

        Self {
            following,
            fuel_consent,
        }
    }
}

impl PartialEq<UpdateUser> for User {
    fn eq(&self, other: &UpdateUser) -> bool {
        let User {
            following,
            fuel_consent,
        } = self;

        Some(following) == other.following.as_ref() && *fuel_consent == other.fuel_consent
    }
}

impl PartialEq<User> for UpdateUser {
    fn eq(&self, other: &User) -> bool {
        other.eq(self)
    }
}
