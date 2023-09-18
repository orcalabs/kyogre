use std::collections::HashSet;

use crate::{DateRange, HaulWeatherOutput, HaulWeatherStatus, Vessel, WeatherQuery};
use crate::{HaulWeatherError, HaulWeatherInbound, HaulWeatherOutbound, SharedState};
use async_trait::async_trait;
use error_stack::ResultExt;
use geo::{coord, Contains};
use machine::Schedule;
use tracing::{event, instrument, Level};

pub struct HaulWeatherState;

#[async_trait]
impl machine::State for HaulWeatherState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        match shared_state.haul_weather_outbound.all_vessels().await {
            Ok(vessels) => {
                if let Err(e) = process_haul_weather(
                    shared_state.haul_weather_outbound.as_ref(),
                    shared_state.haul_weather_inbound.as_ref(),
                    &vessels,
                )
                .await
                {
                    event!(Level::ERROR, "failed to process haul weather: {:?}", e);
                }
            }
            Err(e) => {
                event!(Level::ERROR, "failed to retrieve vessels: {:?}", e);
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

#[instrument(name = "run_haul_weather", skip_all)]
async fn process_haul_weather(
    outbound: &dyn HaulWeatherOutbound,
    inbound: &dyn HaulWeatherInbound,
    vessels: &[Vessel],
) -> error_stack::Result<(), HaulWeatherError> {
    let weather_locations = outbound
        .weather_locations()
        .await
        .change_context(HaulWeatherError)?;

    for vessel in vessels {
        let mmsi = vessel.ais.as_ref().map(|a| a.mmsi);
        let call_sign = vessel.fiskeridir.call_sign.as_ref();

        if mmsi.is_none() && call_sign.is_none() {
            continue;
        }

        let hauls = outbound
            .haul_messages_of_vessel_without_weather(vessel.fiskeridir.id)
            .await
            .change_context(HaulWeatherError)?;

        let mut outputs = Vec::with_capacity(hauls.len());

        for h in hauls {
            let range = DateRange::new(h.start_timestamp, h.stop_timestamp)
                .change_context(HaulWeatherError)?;

            let positions = outbound
                .ais_vms_positions(mmsi, call_sign, &range)
                .await
                .change_context(HaulWeatherError)?;

            if positions.is_empty() {
                outputs.push(HaulWeatherOutput {
                    haul_id: h.haul_id,
                    status: HaulWeatherStatus::Attempted,
                    weather: None,
                });
                continue;
            }

            let locations = positions
                .into_iter()
                .filter_map(|p| {
                    let coord = coord! {x: p.longitude, y: p.latitude};
                    weather_locations
                        .iter()
                        .find(|l| l.polygon.contains(&coord))
                })
                .collect::<HashSet<_>>()
                .into_iter()
                .map(|l| l.id)
                .collect::<Vec<_>>();

            if locations.is_empty() {
                outputs.push(HaulWeatherOutput {
                    haul_id: h.haul_id,
                    status: HaulWeatherStatus::Attempted,
                    weather: None,
                });
                continue;
            }

            let weather = outbound
                .haul_weather(WeatherQuery {
                    start_date: h.start_timestamp,
                    end_date: h.stop_timestamp,
                    weather_location_ids: Some(locations),
                })
                .await
                .change_context(HaulWeatherError)?;

            outputs.push(HaulWeatherOutput {
                haul_id: h.haul_id,
                status: if weather.is_some() {
                    HaulWeatherStatus::Successful
                } else {
                    HaulWeatherStatus::Attempted
                },
                weather,
            });
        }

        inbound
            .add_haul_weather(outputs)
            .await
            .change_context(HaulWeatherError)?;
    }

    Ok(())
}
