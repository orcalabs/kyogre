use error_stack::{report, Report};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::CatchLocationId;

use crate::error::PostgresError;

pub struct CatchLocation {
    pub catch_location_id: String,
    pub polygon: wkb::Decode<Geometry<f64>>,
}

impl TryFrom<CatchLocation> for kyogre_core::CatchLocation {
    type Error = Report<PostgresError>;

    fn try_from(v: CatchLocation) -> Result<Self, Self::Error> {
        let geometry = v
            .polygon
            .geometry
            .ok_or_else(|| report!(PostgresError::DataConversion))?;

        let polygon = match geometry {
            Geometry::Polygon(p) => p,
            _ => return Err(report!(PostgresError::DataConversion)),
        };

        Ok(Self {
            id: CatchLocationId::new_unchecked(v.catch_location_id),
            polygon,
        })
    }
}
