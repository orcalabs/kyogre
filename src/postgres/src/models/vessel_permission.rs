use crate::queries::type_to_i64;
use kyogre_core::FiskeridirVesselId;
use unnest_insert::UnnestInsert;

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "vessel_permissions",
    conflict = "fiskeridir_vessel_id, permission_type"
)]
pub struct VesselPermission {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub permission_type: String,
}

impl From<fiskeridir_rs::VesselPermission> for VesselPermission {
    fn from(value: fiskeridir_rs::VesselPermission) -> Self {
        Self {
            fiskeridir_vessel_id: value.fiskeridir_vessel_id,
            permission_type: value.permission_type,
        }
    }
}
