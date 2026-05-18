use std::fmt::Display;

use fiskeridir_rs::CallSign;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

use crate::FiskeridirVesselId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct FisheryId(i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct BarentswatchUserId(Uuid);

#[derive(Debug, Clone)]
pub struct User {
    pub barentswatch_user_id: BarentswatchUserId,
    pub following: Vec<FiskeridirVesselId>,
    pub fuel_consent: Option<bool>,
    pub selected_vessel: Option<CallSign>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct UpdateUser {
    #[serde(default)]
    pub following: Option<Vec<FiskeridirVesselId>>,
    #[serde(default)]
    pub fuel_consent: Option<bool>,
    #[serde(default)]
    pub selected_vessel: Option<CallSign>,
}

#[derive(Debug, Clone)]
pub struct UpdateSelectedVessel {
    pub selected_vessel: CallSign,
    pub current_associated_vessel: CallSign,
}

impl AsRef<Uuid> for BarentswatchUserId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl Display for FisheryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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

    impl FisheryId {
        pub fn test_new(val: i32) -> Self {
            Self(val)
        }
    }
}
