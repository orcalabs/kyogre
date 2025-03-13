use actix_web::web;
use kyogre_core::{BarentswatchUserId, FiskeridirVesselId};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};

use crate::{Database, error::Result, extractors::BwProfile, response::Response};

#[oasgen(skip(db), tags("User"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn get_user<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<Option<User>>> {
    Ok(Response::new(
        db.get_user(profile.user.id).await?.map(User::from),
    ))
}

#[oasgen(skip(db), tags("User"))]
#[tracing::instrument(skip(db), fields(user_id = profile.tracing_id()))]
pub async fn update_user<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    user: web::Json<User>,
) -> Result<Response<()>> {
    let user = user.into_inner().to_domain_user(profile.user.id);
    db.update_user(&user).await?;
    Ok(Response::new(()))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub following: Vec<FiskeridirVesselId>,
}

impl From<kyogre_core::User> for User {
    fn from(v: kyogre_core::User) -> Self {
        let kyogre_core::User {
            barentswatch_user_id: _,
            following,
        } = v;

        Self { following }
    }
}

impl User {
    pub fn to_domain_user(self, barentswatch_user_id: BarentswatchUserId) -> kyogre_core::User {
        let Self { following } = self;

        kyogre_core::User {
            barentswatch_user_id,
            following,
        }
    }
}
