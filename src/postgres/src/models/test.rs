use crate::queries::{opt_type_to_i32, type_to_i64};
use kyogre_core::{FiskeridirVesselId, Mmsi};
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "all_vessels", conflict = "")]
pub struct VesselConflictInsert {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub call_sign: Option<String>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub mmsi: Option<Mmsi>,
    pub is_manual: bool,
    pub is_active: bool,
}

impl From<kyogre_core::NewVesselConflict> for VesselConflictInsert {
    fn from(value: kyogre_core::NewVesselConflict) -> Self {
        let kyogre_core::NewVesselConflict {
            vessel_id,
            call_sign,
            mmsi,
            is_active,
        } = value;

        Self {
            fiskeridir_vessel_id: vessel_id,
            call_sign: call_sign.map(|v| v.into_inner()),
            mmsi,
            is_active,
            is_manual: true,
        }
    }
}
