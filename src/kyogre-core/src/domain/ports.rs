use serde::Deserialize;

use crate::Coordinates;

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub id: String,
    pub coordinates: Option<Coordinates>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripDockPoints {
    pub start: Vec<PortDockPoint>,
    pub end: Vec<PortDockPoint>,
}

/// Port dock points associated with ports.
/// Originates from Kystverkets register on ports which includes sub-ports.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct PortDockPoint {
    /// Code of the port this dock point belongs to.
    pub code: String,
    /// Unique id within the port.
    pub point_id: i32,
    /// Name of the dock point
    pub name: String,
    /// Port latitude, these coordinates seem to be more accurate than the coordinates
    /// from [crate::Port].
    pub latitude: f64,
    /// Port longitude coordinate.
    pub longitude: f64,
}
