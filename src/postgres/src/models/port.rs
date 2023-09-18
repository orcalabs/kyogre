use bigdecimal::BigDecimal;
use error_stack::{report, Report, Result, ResultExt};
use jurisdiction::Jurisdiction;
use serde::Deserialize;
use std::str::FromStr;
use unnest_insert::UnnestInsert;

use crate::{
    error::{PortCoordinateError, PostgresError},
    queries::decimal_to_float,
};

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "ports", conflict = "port_id")]
pub struct NewPort {
    #[unnest_insert(field_name = "port_id")]
    pub id: String,
    pub name: Option<String>,
    pub nationality: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub id: String,
    pub name: Option<String>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortDockPoint {
    pub port_dock_point_id: i32,
    pub port_id: String,
    pub name: String,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripPorts {
    pub start_port_id: Option<String>,
    pub start_port_name: Option<String>,
    pub start_port_nationality: Option<String>,
    pub start_port_latitude: Option<BigDecimal>,
    pub start_port_longitude: Option<BigDecimal>,
    pub end_port_id: Option<String>,
    pub end_port_name: Option<String>,
    pub end_port_nationality: Option<String>,
    pub end_port_latitude: Option<BigDecimal>,
    pub end_port_longitude: Option<BigDecimal>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct TripDockPoints {
    pub start: Option<String>,
    pub end: Option<String>,
}

impl NewPort {
    pub fn new(id: String, name: Option<String>) -> Result<Self, PostgresError> {
        let jurisdiction = Jurisdiction::from_str(&id[0..2])
            .map_err(|e| report!(PostgresError::DataConversion).attach_printable(format!("{e}")))?;

        Ok(Self {
            id,
            name,
            nationality: jurisdiction.alpha3().to_string(),
        })
    }
}

impl TryFrom<TripDockPoints> for kyogre_core::TripDockPoints {
    type Error = Report<PostgresError>;

    fn try_from(value: TripDockPoints) -> std::result::Result<Self, Self::Error> {
        let start: Vec<kyogre_core::PortDockPoint> = value
            .start
            .map(|v| serde_json::from_str(&v).change_context(PostgresError::DataConversion))
            .transpose()?
            .unwrap_or_default();

        let end: Vec<kyogre_core::PortDockPoint> = value
            .end
            .map(|v| serde_json::from_str(&v).change_context(PostgresError::DataConversion))
            .transpose()?
            .unwrap_or_default();

        Ok(kyogre_core::TripDockPoints { start, end })
    }
}

impl TryFrom<PortDockPoint> for kyogre_core::PortDockPoint {
    type Error = Report<PostgresError>;

    fn try_from(value: PortDockPoint) -> std::result::Result<Self, Self::Error> {
        Ok(kyogre_core::PortDockPoint {
            port_dock_point_id: value.port_dock_point_id,
            port_id: value.port_id,
            name: value.name,
            latitude: decimal_to_float(value.latitude)
                .change_context(PostgresError::DataConversion)?,
            longitude: decimal_to_float(value.longitude)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}

impl TryFrom<Port> for kyogre_core::Port {
    type Error = Report<PostgresError>;

    fn try_from(value: Port) -> std::result::Result<Self, Self::Error> {
        let coordinates = match (value.latitude, value.longitude) {
            (None, None) => Ok(None),
            (Some(lat), Some(lon)) => Ok(Some(kyogre_core::Coordinates {
                latitude: decimal_to_float(lat).change_context(PostgresError::DataConversion)?,
                longitude: decimal_to_float(lon).change_context(PostgresError::DataConversion)?,
            })),
            (None, Some(_)) | (Some(_), None) => {
                Err(report!(PortCoordinateError(value.id.clone()))
                    .change_context(PostgresError::DataConversion))
            }
        }?;

        Ok(kyogre_core::Port {
            id: value.id,
            coordinates,
        })
    }
}

impl TryFrom<TripPorts> for kyogre_core::TripPorts {
    type Error = Report<PostgresError>;

    fn try_from(value: TripPorts) -> std::result::Result<Self, Self::Error> {
        let start =
            if let Some(id) = value.start_port_id {
                match (value.start_port_latitude, value.start_port_longitude) {
                    (None, None) => Ok(Some(kyogre_core::Port {
                        id,
                        coordinates: None,
                    })),
                    (Some(lat), Some(lon)) => Ok(Some(kyogre_core::Port {
                        id,
                        coordinates: Some(kyogre_core::Coordinates {
                            latitude: decimal_to_float(lat)
                                .change_context(PostgresError::DataConversion)?,
                            longitude: decimal_to_float(lon)
                                .change_context(PostgresError::DataConversion)?,
                        }),
                    })),
                    (None, Some(_)) | (Some(_), None) => Err(report!(PortCoordinateError(id))
                        .change_context(PostgresError::DataConversion)),
                }
            } else {
                Ok(None)
            }?;

        let end =
            if let Some(id) = value.end_port_id {
                match (value.end_port_latitude, value.end_port_longitude) {
                    (None, None) => Ok(Some(kyogre_core::Port {
                        id,
                        coordinates: None,
                    })),
                    (Some(lat), Some(lon)) => Ok(Some(kyogre_core::Port {
                        id,
                        coordinates: Some(kyogre_core::Coordinates {
                            latitude: decimal_to_float(lat)
                                .change_context(PostgresError::DataConversion)?,
                            longitude: decimal_to_float(lon)
                                .change_context(PostgresError::DataConversion)?,
                        }),
                    })),
                    (None, Some(_)) | (Some(_), None) => Err(report!(PortCoordinateError(id))
                        .change_context(PostgresError::DataConversion)),
                }
            } else {
                Ok(None)
            }?;

        Ok(kyogre_core::TripPorts { start, end })
    }
}
