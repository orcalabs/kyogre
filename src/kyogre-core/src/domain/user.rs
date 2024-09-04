use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::FiskeridirVesselId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct BarentswatchUserId(Uuid);

#[derive(Debug, Clone)]
pub struct User {
    pub barentswatch_user_id: BarentswatchUserId,
    pub following: Vec<FiskeridirVesselId>,
}

impl AsRef<Uuid> for BarentswatchUserId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl BarentswatchUserId {
    pub fn test_new() -> Self {
        Self(Uuid::new_v4())
    }
}
