use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::FiskeridirVesselId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BarentswatchUserId(pub Uuid);

#[derive(Debug, Clone)]
pub struct User {
    pub barentswatch_user_id: BarentswatchUserId,
    pub following: Vec<FiskeridirVesselId>,
}
