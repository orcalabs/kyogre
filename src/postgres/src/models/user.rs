use kyogre_core::{BarentswatchUserId, FiskeridirVesselId};
use uuid::Uuid;

use crate::error::PostgresErrorWrapper;

#[derive(Debug, Clone)]
pub struct User {
    pub barentswatch_user_id: Uuid,
    pub following: Vec<i64>,
}

impl TryFrom<User> for kyogre_core::User {
    type Error = PostgresErrorWrapper;

    fn try_from(v: User) -> Result<Self, Self::Error> {
        Ok(Self {
            barentswatch_user_id: BarentswatchUserId(v.barentswatch_user_id),
            following: v.following.into_iter().map(FiskeridirVesselId).collect(),
        })
    }
}
