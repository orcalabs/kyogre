use std::cmp::min;
use std::collections::HashSet;
use std::sync::Arc;

use crate::SharedState;
use crate::{
    DateRange, HaulWeatherOutput, HaulWeatherStatus, OceanClimateQuery, Vessel, WeatherQuery,
    error::Result,
};
use async_channel::bounded;
use async_trait::async_trait;
use geo::{Contains, coord};
use kyogre_core::{HaulWeatherOutbound, WeatherLocation};
use machine::Schedule;
use tokio::sync::mpsc::channel;
use tracing::{error, instrument};

pub struct HaulWeatherState;

#[async_trait]
impl machine::State for HaulWeatherState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        let shared_state = Arc::new(shared_state);

        if let Err(e) = process_haul_weather(shared_state.clone()).await {
            error!("failed to process haul weather: {e:?}");
        }

        match Arc::into_inner(shared_state) {
            Some(shared_state) => shared_state,
            None => {
                error!("failed to run haul weather: shared_state returned had multiple references");
                panic!()
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

#[instrument(name = "run_haul_weather", skip_all)]
async fn process_haul_weather(shared_state: Arc<SharedState>) -> Result<()> {
    let vessels = shared_state.haul_weather_outbound.all_vessels().await?;

    if vessels.is_empty() {
        return Ok(());
    }

    let weather_locations = shared_state
        .haul_weather_outbound
        .weather_locations()
        .await?;

    let weather_locations = Arc::new(weather_locations);

    let num_vessels = vessels.len();
    let num_workers = min(shared_state.num_workers as usize, num_vessels);

    let (master_tx, mut master_rx) = channel::<Result<_>>(10);
    let (worker_tx, worker_rx) = bounded::<Vessel>(num_vessels);

    for v in vessels {
        worker_tx.try_send(v).unwrap();
    }

    let mut workers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        workers.push(tokio::spawn({
            let master_tx = master_tx.clone();
            let worker_rx = worker_rx.clone();
            let shared_state = shared_state.clone();
            let weather_locations = weather_locations.clone();

            async move {
                while let Ok(vessel) = worker_rx.try_recv() {
                    if let Some(outputs) = process(
                        &vessel,
                        &weather_locations,
                        shared_state.haul_weather_outbound.as_ref(),
                    )
                    .await
                    .transpose()
                    {
                        master_tx.send(outputs).await.unwrap();
                    }
                }
            }
        }));
    }

    drop(master_tx);

    while let Some(value) = master_rx.recv().await {
        match value {
            Ok(outputs) => {
                if let Err(e) = shared_state
                    .haul_weather_inbound
                    .add_haul_weather(outputs)
                    .await
                {
                    error!("failed to store haul weather output: {e:?}");
                }
            }
            Err(e) => error!("failed to process haul weather: {e:?}"),
        }
    }

    for w in workers {
        w.await.unwrap();
    }

    Ok(())
}

async fn process(
    vessel: &Vessel,
    weather_locations: &[WeatherLocation],
    outbound: &dyn HaulWeatherOutbound,
) -> Result<Option<Vec<HaulWeatherOutput>>> {
    let mmsi = vessel.ais.as_ref().map(|a| a.mmsi);
    let call_sign = vessel.fiskeridir.call_sign.as_ref();

    if mmsi.is_none() && call_sign.is_none() {
        return Ok(None);
    }

    let hauls = outbound
        .haul_messages_of_vessel_without_weather(vessel.fiskeridir.id)
        .await?;

    let mut outputs = Vec::with_capacity(hauls.len());

    for h in hauls {
        let range = DateRange::new(h.start_timestamp, h.stop_timestamp)?;

        let positions = outbound.ais_vms_positions(mmsi, call_sign, &range).await?;

        if positions.is_empty() {
            outputs.push(HaulWeatherOutput {
                haul_id: h.haul_id,
                status: HaulWeatherStatus::Attempted,
                weather: None,
                ocean_climate: None,
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
                ocean_climate: None,
            });
            continue;
        }

        let weather = outbound
            .haul_weather(WeatherQuery {
                start_date: h.start_timestamp,
                end_date: h.stop_timestamp,
                weather_location_ids: Some(locations.clone()),
            })
            .await?;

        let ocean_climate = outbound
            .haul_ocean_climate(OceanClimateQuery {
                start_date: h.start_timestamp,
                end_date: h.stop_timestamp,
                depths: Some(vec![0]),
                weather_location_ids: Some(locations),
            })
            .await?;

        outputs.push(HaulWeatherOutput {
            haul_id: h.haul_id,
            status: if weather.is_some() || ocean_climate.is_some() {
                HaulWeatherStatus::Successful
            } else {
                HaulWeatherStatus::Attempted
            },
            weather,
            ocean_climate,
        });
    }

    Ok(Some(outputs))
}
