use chrono::{DateTime, Utc};
use kyogre_core::{FiskeridirVesselId, VesselEventData, VesselEventType};
use serde::Deserialize;

use crate::error::{Error, MissingValueSnafu};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VesselEvent {
    pub vessel_event_id: i64,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub report_timestamp: DateTime<Utc>,
    pub occurence_timestamp: Option<DateTime<Utc>>,
    pub vessel_event_type_id: VesselEventType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VesselEventDetailed {
    pub vessel_event_id: i64,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub report_timestamp: DateTime<Utc>,
    pub vessel_event_type_id: VesselEventType,
    pub departure_port_id: Option<String>,
    pub arrival_port_id: Option<String>,
    pub port_id: Option<String>,
    pub estimated_timestamp: Option<DateTime<Utc>>,
}

impl From<VesselEvent> for kyogre_core::VesselEvent {
    fn from(v: VesselEvent) -> kyogre_core::VesselEvent {
        let VesselEvent {
            vessel_event_id,
            fiskeridir_vessel_id,
            report_timestamp,
            occurence_timestamp,
            vessel_event_type_id,
        } = v;

        Self {
            event_id: vessel_event_id as u64,
            vessel_id: fiskeridir_vessel_id,
            report_timestamp,
            event_type: vessel_event_type_id,
            occurence_timestamp,
        }
    }
}

impl TryFrom<VesselEventDetailed> for kyogre_core::VesselEventDetailed {
    type Error = Error;

    fn try_from(v: VesselEventDetailed) -> Result<kyogre_core::VesselEventDetailed, Self::Error> {
        let VesselEventDetailed {
            vessel_event_id,
            fiskeridir_vessel_id,
            report_timestamp,
            vessel_event_type_id,
            departure_port_id,
            arrival_port_id,
            port_id,
            estimated_timestamp,
        } = v;

        let event_data = match vessel_event_type_id {
            VesselEventType::Landing => VesselEventData::Landing,
            VesselEventType::ErsDca => VesselEventData::ErsDca,
            VesselEventType::ErsTra => VesselEventData::ErsTra,
            VesselEventType::Haul => VesselEventData::Haul,
            VesselEventType::ErsPor => VesselEventData::ErsPor {
                port_id: arrival_port_id.or(port_id),
                estimated_timestamp: estimated_timestamp
                    .ok_or_else(|| MissingValueSnafu.build())?,
            },
            VesselEventType::ErsDep => VesselEventData::ErsDep {
                port_id: departure_port_id.or(port_id),
                estimated_timestamp: estimated_timestamp
                    .ok_or_else(|| MissingValueSnafu.build())?,
            },
        };

        Ok(kyogre_core::VesselEventDetailed {
            event_id: vessel_event_id as u64,
            vessel_id: fiskeridir_vessel_id,
            timestamp: report_timestamp,
            event_type: vessel_event_type_id,
            event_data,
        })
    }
}
