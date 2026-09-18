use crate::FiskeridirVesselId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct VesselPermission {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub permission_type: String,
}
