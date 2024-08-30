use actix_web::web;
use kyogre_core::{BarentswatchUserId, FiskeridirVesselId};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

use crate::{error::ApiError, extractors::BwProfile, response::Response, Database};

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
) -> Result<Response<Option<User>>, ApiError> {
    Ok(Response::new(
        db.get_user(BarentswatchUserId(profile.user.id))
            .await
            .map_err(|e| {
                error!("failed to get user: {e:?}");
                ApiError::InternalServerError
            })?
            .map(User::from),
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
) -> Result<Response<()>, ApiError> {
    let user = user
        .into_inner()
        .to_domain_user(BarentswatchUserId(profile.user.id));

    db.update_user(user)
        .await
        .map_err(|e| {
            error!("failed to update user: {e:?}");
            ApiError::InternalServerError
        })
        .map(|_| Response::new(()))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[schema(value_type = Vec<i64>)]
    pub following: Vec<FiskeridirVesselId>,
}

impl From<kyogre_core::User> for User {
    fn from(v: kyogre_core::User) -> Self {
        Self {
            following: v.following,
        }
    }
}

impl User {
    pub fn to_domain_user(self, barentswatch_user_id: BarentswatchUserId) -> kyogre_core::User {
        kyogre_core::User {
            barentswatch_user_id,
            following: self.following,
        }
    }
}
