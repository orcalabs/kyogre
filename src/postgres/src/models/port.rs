use jurisdiction::Jurisdiction;
use serde::Deserialize;
use std::str::FromStr;
use unnest_insert::UnnestInsert;

use crate::error::{Error, JurisdictionSnafu, MissingValueSnafu, Result};

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "ports", conflict = "port_id")]
pub struct NewPort<'a> {
    #[unnest_insert(field_name = "port_id")]
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub nationality: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub id: String,
    pub name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortDockPoint {
    pub port_dock_point_id: i32,
    pub port_id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripPorts {
    pub start_port_id: Option<String>,
    pub start_port_name: Option<String>,
    pub start_port_nationality: Option<String>,
    pub start_port_latitude: Option<f64>,
    pub start_port_longitude: Option<f64>,
    pub end_port_id: Option<String>,
    pub end_port_name: Option<String>,
    pub end_port_nationality: Option<String>,
    pub end_port_latitude: Option<f64>,
    pub end_port_longitude: Option<f64>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct TripDockPoints {
    pub start: Option<String>,
    pub end: Option<String>,
}

impl<'a> NewPort<'a> {
    pub fn new(id: &'a str, name: Option<&'a str>) -> Result<Self> {
        let jurisdiction = Jurisdiction::from_str(&id[0..2]).map_err(|e| {
            JurisdictionSnafu {
                error_stringified: e.to_string(),
                data: id,
            }
            .build()
        })?;

        Ok(Self {
            id,
            name,
            nationality: jurisdiction.alpha3().to_string(),
        })
    }
}

impl TryFrom<TripDockPoints> for kyogre_core::TripDockPoints {
    type Error = Error;

    fn try_from(value: TripDockPoints) -> std::result::Result<Self, Self::Error> {
        let start: Vec<kyogre_core::PortDockPoint> = value
            .start
            .map(|v| serde_json::from_str(&v))
            .transpose()?
            .unwrap_or_default();

        let end: Vec<kyogre_core::PortDockPoint> = value
            .end
            .map(|v| serde_json::from_str(&v))
            .transpose()?
            .unwrap_or_default();

        Ok(kyogre_core::TripDockPoints { start, end })
    }
}

impl TryFrom<PortDockPoint> for kyogre_core::PortDockPoint {
    type Error = Error;

    fn try_from(value: PortDockPoint) -> std::result::Result<Self, Self::Error> {
        Ok(kyogre_core::PortDockPoint {
            port_dock_point_id: value.port_dock_point_id,
            port_id: value.port_id,
            name: value.name,
            latitude: value.latitude,
            longitude: value.longitude,
        })
    }
}

impl TryFrom<Port> for kyogre_core::Port {
    type Error = Error;

    fn try_from(value: Port) -> std::result::Result<Self, Self::Error> {
        let coordinates = match (value.latitude, value.longitude) {
            (None, None) => None,
            (Some(lat), Some(lon)) => Some(kyogre_core::Coordinates {
                latitude: lat,
                longitude: lon,
            }),
            (None, Some(_)) | (Some(_), None) => return MissingValueSnafu.fail(),
        };

        Ok(kyogre_core::Port {
            id: value.id,
            coordinates,
        })
    }
}

impl TryFrom<TripPorts> for kyogre_core::TripPorts {
    type Error = Error;

    fn try_from(value: TripPorts) -> std::result::Result<Self, Self::Error> {
        let start = if let Some(id) = value.start_port_id {
            match (value.start_port_latitude, value.start_port_longitude) {
                (None, None) => Some(kyogre_core::Port {
                    id,
                    coordinates: None,
                }),
                (Some(lat), Some(lon)) => Some(kyogre_core::Port {
                    id,
                    coordinates: Some(kyogre_core::Coordinates {
                        latitude: lat,
                        longitude: lon,
                    }),
                }),
                (None, Some(_)) | (Some(_), None) => return MissingValueSnafu.fail(),
            }
        } else {
            None
        };

        let end = if let Some(id) = value.end_port_id {
            match (value.end_port_latitude, value.end_port_longitude) {
                (None, None) => Some(kyogre_core::Port {
                    id,
                    coordinates: None,
                }),
                (Some(lat), Some(lon)) => Some(kyogre_core::Port {
                    id,
                    coordinates: Some(kyogre_core::Coordinates {
                        latitude: lat,
                        longitude: lon,
                    }),
                }),
                (None, Some(_)) | (Some(_), None) => return MissingValueSnafu.fail(),
            }
        } else {
            None
        };

        Ok(kyogre_core::TripPorts { start, end })
    }
}
