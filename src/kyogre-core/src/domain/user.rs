use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

use crate::FiskeridirVesselId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
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

impl Display for BarentswatchUserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "test")]
mod test {
    use super::*;

    impl BarentswatchUserId {
        pub fn test_new() -> Self {
            Self(Uuid::new_v4())
        }
    }
}
