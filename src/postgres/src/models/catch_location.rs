use crate::error::{Error, MissingValueSnafu};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::CatchLocationId;

pub struct CatchLocation {
    pub id: CatchLocationId,
    pub polygon: wkb::Decode<Geometry<f64>>,
    pub latitude: f64,
    pub longitude: f64,
    pub weather_location_ids: Vec<i64>,
}

impl TryFrom<CatchLocation> for kyogre_core::CatchLocation {
    type Error = Error;

    fn try_from(v: CatchLocation) -> Result<Self, Self::Error> {
        let CatchLocation {
            id,
            polygon,
            latitude,
            longitude,
            weather_location_ids,
        } = v;

        let geometry = polygon
            .geometry
            .ok_or_else(|| MissingValueSnafu {}.build())?;

        let polygon = match geometry {
            Geometry::Polygon(p) => p,
            _ => return MissingValueSnafu {}.fail(),
        };

        Ok(Self {
            id,
            polygon,
            latitude,
            longitude,
            weather_location_ids,
        })
    }
}
