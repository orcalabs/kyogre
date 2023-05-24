use chrono::{DateTime, Utc};
use error_stack::Report;
use kyogre_core::{FiskeridirVesselId, VesselEventData, VesselEventType};

use crate::error::PostgresError;

#[derive(Debug, Clone, PartialEq)]
pub struct VesselEvent {
    pub vessel_event_id: i64,
    pub fiskeridir_vessel_id: i32,
    pub timestamp: DateTime<Utc>,
    pub vessel_event_type_id: VesselEventType,
}

impl From<VesselEvent> for kyogre_core::VesselEvent {
    fn from(v: VesselEvent) -> kyogre_core::VesselEvent {
        kyogre_core::VesselEvent {
            event_id: v.vessel_event_id as u64,
            vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id as i64),
            timestamp: v.timestamp,
            event_type: v.vessel_event_type_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VesselEventDetailed {
    pub vessel_event_id: i64,
    pub fiskeridir_vessel_id: i32,
    pub timestamp: DateTime<Utc>,
    pub vessel_event_type_id: VesselEventType,
    pub departure_port_id: Option<String>,
    pub arrival_port_id: Option<String>,
    pub port_id: Option<String>,
    pub estimated_timestamp: Option<DateTime<Utc>>,
}

impl TryFrom<VesselEventDetailed> for kyogre_core::VesselEventDetailed {
    type Error = Report<PostgresError>;

    fn try_from(v: VesselEventDetailed) -> Result<kyogre_core::VesselEventDetailed, Self::Error> {
        let event_data: Result<VesselEventData, Report<PostgresError>> = match v
            .vessel_event_type_id
        {
            VesselEventType::Landing => Ok(VesselEventData::Landing),
            VesselEventType::ErsDca => Ok(VesselEventData::ErsDca),
            VesselEventType::ErsTra => Ok(VesselEventData::ErsTra),
            VesselEventType::ErsPor => Ok(VesselEventData::ErsPor {
                port_id: if v.arrival_port_id.is_some() {
                    v.arrival_port_id
                } else {
                    v.port_id
                },
                estimated_timestamp: v.estimated_timestamp.ok_or(PostgresError::DataConversion)?,
            }),
            VesselEventType::ErsDep => Ok(VesselEventData::ErsDep {
                port_id: if v.departure_port_id.is_some() {
                    v.departure_port_id
                } else {
                    v.port_id
                },
                estimated_timestamp: v.estimated_timestamp.ok_or(PostgresError::DataConversion)?,
            }),
        };

        Ok(kyogre_core::VesselEventDetailed {
            event_id: v.vessel_event_id as u64,
            vessel_id: FiskeridirVesselId(v.fiskeridir_vessel_id as i64),
            timestamp: v.timestamp,
            event_type: v.vessel_event_type_id,
            event_data: event_data?,
        })
    }
}
