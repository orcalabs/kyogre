use crate::queries::enum_to_i32;
use kyogre_core::{DeliveryPointSourceId, DeliveryPointType};
use unnest_insert::UnnestInsert;

#[derive(UnnestInsert)]
#[unnest_insert(table_name = "delivery_points", conflict = "delivery_point_id")]
pub struct NewDeliveryPoint {
    pub delivery_point_id: String,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub delivery_point_type_id: DeliveryPointType,
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub delivery_point_source_id: DeliveryPointSourceId,
}

impl From<fiskeridir_rs::DeliveryPointId> for NewDeliveryPoint {
    fn from(v: fiskeridir_rs::DeliveryPointId) -> Self {
        let delivery_point_type_id = if v.broenbaat() {
            DeliveryPointType::Broenbaat
        } else {
            DeliveryPointType::Ukjent
        };

        Self {
            delivery_point_id: v.into_inner(),
            delivery_point_type_id,
            delivery_point_source_id: DeliveryPointSourceId::NoteData,
        }
    }
}
