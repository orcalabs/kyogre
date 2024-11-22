use actix_web::web;
use kyogre_core::{BarentswatchUserId, FiskeridirVesselId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    error::{ErrorResponse, Result},
    extractors::BwProfile,
    response::Response,
    Database,
};

#[utoipa::path(
    get,
    path = "/user",
    responses(
        (status = 200, description = "user information", body = User),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn get_user<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
) -> Result<Response<Option<User>>> {
    Ok(Response::new(
        db.get_user(profile.user.id).await?.map(User::from),
    ))
}

#[utoipa::path(
    put,
    path = "/user",
    request_body(
        content = User,
        content_type = "application/json",
        description = "updated user information",
    ),
    responses(
        (status = 200, description = "update successfull"),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn update_user<T: Database + 'static>(
    db: web::Data<T>,
    profile: BwProfile,
    user: web::Json<User>,
) -> Result<Response<()>> {
    let user = user.into_inner().to_domain_user(profile.user.id);
    db.update_user(&user).await?;
    Ok(Response::new(()))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[schema(value_type = Vec<i64>)]
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
