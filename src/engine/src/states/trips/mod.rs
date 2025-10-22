use std::{cmp::min, collections::HashMap, sync::Arc};

use async_channel::bounded;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use machine::Schedule;
use tokio::{select, sync::mpsc::channel, task::JoinSet};
use tracing::{error, info};

use crate::{error::Result, *};

mod computation_step;

static UNPROCESSED_TRIPS_BATCH_SIZE: u32 = 100;

pub use computation_step::*;

pub struct TripsState;

#[derive(Debug, Default)]
pub struct TripsReport {
    pub num_trips: u32,
    pub num_conflicts: u32,
    pub num_no_prior_state: u32,
    pub num_vessels: u32,
    pub num_failed: u32,
    pub num_reset: u32,
}

#[derive(Debug)]
pub struct TripProcessingOutcome {
    pub num_trips: u32,
    pub state: AssemblerState,
}

#[derive(Debug)]
pub struct TripAssembly {
    pub trips: Vec<NewTrip>,
    pub conflict_strategy: TripsConflictStrategy,
    pub prior_trip_calculation_time: Option<DateTime<Utc>>,
    pub trip_assembler_id: TripAssemblerId,
    pub conflict: Option<TripAssemblerConflict>,
    pub new_trip_events: Vec<MinimalVesselEvent>,
    pub prior_trip_events: Vec<MinimalVesselEvent>,
}

impl std::ops::Add<TripProcessingOutcome> for TripsReport {
    type Output = TripsReport;

    fn add(mut self, rhs: TripProcessingOutcome) -> Self::Output {
        self.num_trips += rhs.num_trips;
        self.num_vessels += 1;
        match rhs.state {
            AssemblerState::Conflict(_) => self.num_conflicts += 1,
            AssemblerState::NoPriorState => self.num_no_prior_state += 1,
            AssemblerState::TripCalculationTimer(_) => (),
            AssemblerState::QueuedReset => self.num_reset += 1,
        }
        self
    }
}

#[async_trait]
impl machine::State for TripsState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        let shared_state = Arc::new(shared_state);

        match run_state(shared_state.clone()).await {
            Err(e) => error!("failed to run trips pipeline: {e:?}"),
            Ok(r) => {
                info!(
                    "num_conflicts: {}, num_vessels: {}, num_no_prior_state: {}
                       num_trips: {}, num_failed: {}, num_reset: {}",
                    r.num_conflicts,
                    r.num_vessels,
                    r.num_no_prior_state,
                    r.num_trips,
                    r.num_failed,
                    r.num_reset
                );
            }
        }

        match Arc::into_inner(shared_state) {
            Some(shared_state) => shared_state,
            None => {
                error!(
                    "failed to run trips pipeline: shared_state returned had multiple references"
                );
                panic!()
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

enum MasterTask {
    New(Vessel, Result<(TripProcessingOutcome, Option<TripSet>)>),
    Unprocessed(Vessel, Result<Vec<TripUpdate>>),
}

enum WorkerTask {
    New(Vessel),
    Unprocessed(Vessel),
}

async fn run_state(shared_state: Arc<SharedState>) -> Result<TripsReport> {
    // For faster local iteration we don't need to update trip assemblers
    // since it is a relatively expensive query.
    if shared_state.local_processing_vessels.is_none() {
        shared_state
            .trip_pipeline_inbound
            .update_preferred_trip_assemblers()
            .await?;
    }

    shared_state
        .trip_pipeline_inbound
        .check_for_out_of_order_vms_insertion()
        .await?;

    shared_state
        .trip_pipeline_inbound
        .delete_uncommited_trips()
        .await?;

    let mut vessels = shared_state
        .trip_assembler_outbound_port
        .all_vessels()
        .await?;

    if let Some(ids) = &shared_state.local_processing_vessels {
        vessels.retain(|v| ids.contains(&v.id()));
    }

    if vessels.is_empty() {
        return Ok(Default::default());
    }

    let ports: HashMap<String, Port> = shared_state
        .trip_assembler_outbound_port
        .ports()
        .await?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect::<HashMap<String, Port>>();

    let mut dock_points_map: HashMap<String, Vec<PortDockPoint>> = HashMap::new();
    let dock_points = shared_state
        .trip_assembler_outbound_port
        .dock_points()
        .await?;

    for d in dock_points {
        dock_points_map
            .entry(d.port_id.clone())
            .and_modify(|v| v.push(d.clone()))
            .or_insert_with(|| vec![d]);
    }

    let ports = Arc::new(ports);
    let dock_points = Arc::new(dock_points_map);

    let num_vessels = vessels.len();
    let num_workers = min(num_vessels, shared_state.num_workers as usize);

    let (master_tx, mut master_rx) = channel(5);
    let (worker_tx, worker_rx) = bounded(num_vessels);

    let mut workers = JoinSet::new();

    for _ in 0..num_workers {
        let master_tx = master_tx.clone();
        let worker_rx = worker_rx.clone();
        let shared_state = shared_state.clone();
        let ports = ports.clone();
        let dock_points = dock_points.clone();

        workers.spawn(async move {
            while let Ok(task) = worker_rx.recv().await {
                let result = match task {
                    WorkerTask::New(vessel) => {
                        let result =
                            process_vessel(&shared_state, &vessel, &ports, &dock_points).await;
                        MasterTask::New(vessel, result)
                    }
                    WorkerTask::Unprocessed(vessel) => {
                        let result =
                            process_unprocessed_trips(&shared_state, &vessel, &ports, &dock_points)
                                .await;
                        MasterTask::Unprocessed(vessel, result)
                    }
                };
                master_tx.send(result).await.unwrap()
            }
        });
    }

    for v in vessels {
        worker_tx.try_send(WorkerTask::New(v)).unwrap();
    }

    let mut trips_report = TripsReport::default();

    let mut exit = false;
    let mut completed = 0;
    let mut errored = 0;

    while !exit && completed + errored < num_vessels && errored < num_workers {
        select! {
            _ = workers.join_next() => {
                errored += 1;
            }
            Some(task) = master_rx.recv() => {
                match task {
                    MasterTask::New(vessel, result) => {
                        match result {
                            Ok((TripProcessingOutcome { num_trips: 0, state: AssemblerState::QueuedReset }, None)) => {
                                    if let Err(e) =
                                        shared_state.trip_pipeline_inbound.nuke_trips(vessel.fiskeridir.id).await
                                    {
                                        error!(
                                            "failed to nuke trips for vessel: {}, err: {e:?}",
                                            vessel.fiskeridir.id,
                                        );
                                    }
                            }
                            Ok((report, trips)) => {
                                trips_report = trips_report + report;

                                if let Some(trips) = trips {
                                    if let Err(e) = shared_state.trip_pipeline_inbound.add_trip_set(trips).await {
                                        error!(
                                            "failed to store trips for vessel: {}, err: {e:?}",
                                            vessel.fiskeridir.id,
                                        );
                                    }
                                } else {
                                    // Regardless if we had no trips to add we need to set the current
                                    // trip to add any new hauls or fishing facilites that might have
                                    // been added.
                                    match vessel.preferred_trip_assembler {
                                        TripAssemblerId::Landings => (),
                                        TripAssemblerId::Ers => {
                                            if let Err(e) = shared_state.trip_pipeline_inbound.set_current_trip(vessel.fiskeridir.id).await {
                                                error!(
                                                    "failed to set current trip for vessel: {}, err: {e:?}",
                                                    vessel.fiskeridir.id,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => error!(
                                "failed to run trips pipeline for vessel: {}, err: {e:?}",
                                vessel.fiskeridir.id,
                            ),
                        }

                        worker_tx.try_send(WorkerTask::Unprocessed(vessel)).unwrap();
                    }
                    MasterTask::Unprocessed(vessel, result) => {
                        match result {
                            Ok(updates) => {
                                let more_updates_to_process = !updates.is_empty();
                                for update in updates {
                                    let trip_id = update.trip_id;
                                    if let Err(e) =
                                        shared_state.trip_pipeline_inbound.update_trip(update).await
                                    {
                                        error!("failed to update trip_id: {trip_id}, err: {e:?}");
                                    }
                                }

                                if let Err(e) = shared_state
                                    .trip_pipeline_inbound
                                    .refresh_detailed_trips(vessel.fiskeridir.id)
                                    .await
                                {
                                    error!(
                                        "failed to refresh detailed trips for vessel: {}, err: {e:?}",
                                        vessel.fiskeridir.id,
                                    );
                                }

                                // Processing unprocessed trips occurs in batches and should stop
                                // once there are no more updates to be processed which is
                                // indicated by the update vec being empty.
                                if more_updates_to_process {
                                    worker_tx.try_send(WorkerTask::Unprocessed(vessel)).unwrap();
                                }
                            }
                            Err(e) => error!(
                                "failed to process unprocessed trips for vessel: {}, err: {e:?}",
                                vessel.fiskeridir.id,
                            ),
                        }

                        completed += 1;
                        if completed % 1_000 == 0 {
                            info!("processed {completed}/{num_vessels} vessels");
                        }
                    }
                }
            }
            else => {
                exit = true;
            }
        }
    }

    workers.shutdown().await;

    if exit {
        error!("trips processing master channel exited for an unexpected reason");
    } else {
        info!(
            "vessels completed: {completed}/{num_vessels}, workers exited: {errored}/{num_workers}"
        );
    }

    Ok(trips_report)
}

async fn process_vessel(
    shared: &SharedState,
    vessel: &Vessel,
    ports: &HashMap<String, Port>,
    dock_points: &HashMap<String, Vec<PortDockPoint>>,
) -> Result<(TripProcessingOutcome, Option<TripSet>)> {
    let assembler_impl = shared.assembler_id_to_impl(vessel.preferred_trip_assembler);
    let (outcome, trips) = run_trip_assembler(
        vessel,
        shared.trip_assembler_outbound_port.as_ref(),
        assembler_impl,
    )
    .await?;

    if let Some(trips) = trips {
        let mut output = TripSet {
            fiskeridir_vessel_id: vessel.fiskeridir.id,
            conflict_strategy: trips.conflict_strategy,
            trip_assembler_id: assembler_impl.assembler_id(),
            values: vec![],
            conflict: trips.conflict,
            new_trip_events: trips.new_trip_events,
            prior_trip_events: trips.prior_trip_events,
            prior_trip_calculation_time: trips.prior_trip_calculation_time,
            queued_reset: outcome.state == AssemblerState::QueuedReset,
        };
        for t in trips.trips {
            let trip_id = shared.trip_pipeline_inbound.reserve_trip_id().await?;
            let mut unit = TripProcessingUnit {
                precision_outcome: None,
                distance_output: None,
                start_port: t
                    .start_port_code
                    .as_ref()
                    .and_then(|v| ports.get(v).cloned()),
                end_port: t.end_port_code.as_ref().and_then(|v| ports.get(v).cloned()),
                start_dock_points: t
                    .start_port_code
                    .as_ref()
                    .and_then(|v| dock_points.get(v).cloned())
                    .unwrap_or_default(),
                end_dock_points: t
                    .end_port_code
                    .as_ref()
                    .and_then(|v| dock_points.get(v).cloned())
                    .unwrap_or_default(),
                positions: shared
                    .trips_precision_outbound_port
                    .ais_vms_positions_with_inside_haul(
                        vessel.id(),
                        vessel.mmsi(),
                        vessel.fiskeridir_call_sign(),
                        &t.period_extended,
                    )
                    .await?,
                vessel_id: vessel.fiskeridir.id,
                trip_assembler_id: output.trip_assembler_id,
                position_layers_output: None,
                trip: t,
                trip_id,
            };

            for step in TRIP_COMPUTATION_STEPS.iter() {
                unit = step.run(shared, vessel, unit).await?;
            }

            let pruned_positions = unit.position_layers_output.take();
            let track_coverage = pruned_positions
                .as_ref()
                .map(|p| p.track_coverage)
                .unwrap_or(0.);

            shared
                .trip_pipeline_inbound
                .add_trip_positions(unit.trip_id, &unit.positions, pruned_positions)
                .await?;

            let TripProcessingUnit {
                vessel_id,
                trip,
                trip_id,
                trip_assembler_id,
                start_port,
                end_port,
                start_dock_points,
                end_dock_points,
                positions: _,
                precision_outcome,
                distance_output,
                position_layers_output: _,
            } = unit;

            output.values.push(TripToInsert {
                vessel_id,
                trip,
                trip_id,
                trip_assembler_id,
                start_port,
                end_port,
                start_dock_points,
                end_dock_points,
                precision_outcome,
                distance_output,
                track_coverage,
            });
        }

        Ok((outcome, Some(output)))
    } else {
        Ok((outcome, None))
    }
}

async fn run_trip_assembler(
    vessel: &Vessel,
    adapter: &dyn TripAssemblerOutboundPort,
    assembler: &dyn TripAssembler,
) -> Result<(TripProcessingOutcome, Option<TripAssembly>)> {
    let timer = adapter
        .trip_calculation_timer(vessel.fiskeridir.id, assembler.assembler_id())
        .await?;

    let conflict = timer.as_ref().and_then(|v| v.conflict.clone());
    let prior_trip_calculation_time = timer.as_ref().map(|t| t.timestamp);

    let state = if let Some(timer) = timer.clone() {
        match (timer.conflict, timer.queued_reset) {
            (_, true) => AssemblerState::QueuedReset,
            (Some(c), false) => AssemblerState::Conflict(c),
            (None, false) => AssemblerState::TripCalculationTimer(timer.timestamp),
        }
    } else {
        AssemblerState::NoPriorState
    };

    let (start_and_end_event, succeeding_events) = match &state {
        AssemblerState::Conflict(c) => {
            new_vessel_events(
                vessel.fiskeridir.id,
                adapter,
                assembler.assembler_id(),
                TripSearchTimestamp::Conflict(c.timestamp),
            )
            .await?
        }
        AssemblerState::TripCalculationTimer(t) => {
            new_vessel_events(
                vessel.fiskeridir.id,
                adapter,
                assembler.assembler_id(),
                TripSearchTimestamp::TripCalculationTimer(*t),
            )
            .await?
        }
        AssemblerState::NoPriorState | AssemblerState::QueuedReset => {
            let succeeding_events = adapter
                .all_vessel_events(vessel.fiskeridir.id, assembler.assembler_id())
                .await?;
            (vec![], succeeding_events)
        }
    };

    let new_trip_events: Vec<MinimalVesselEvent> = succeeding_events
        .iter()
        .map(MinimalVesselEvent::from)
        .collect();
    let prior_trip_events: Vec<MinimalVesselEvent> = start_and_end_event
        .iter()
        .map(MinimalVesselEvent::from)
        .collect();

    let trips = assembler
        .assemble(start_and_end_event, succeeding_events)
        .await?;

    if let Some(trips) = trips {
        let conflict_strategy = match (&state, trips.conflict_strategy) {
            (AssemblerState::NoPriorState, Some(r))
            | (AssemblerState::TripCalculationTimer(_), Some(r)) => r,
            (AssemblerState::NoPriorState, None)
            | (AssemblerState::TripCalculationTimer(_), None) => TripsConflictStrategy::Error,
            (AssemblerState::Conflict(_), Some(v)) => v,
            (AssemblerState::Conflict(v), _) => TripsConflictStrategy::Replace {
                conflict: v.timestamp,
            },
            (AssemblerState::QueuedReset, _) => TripsConflictStrategy::ReplaceAll,
        };

        Ok((
            TripProcessingOutcome {
                num_trips: trips.new_trips.len() as u32,
                state,
            },
            Some(TripAssembly {
                trips: trips.new_trips,
                conflict_strategy,
                trip_assembler_id: assembler.assembler_id(),
                prior_trip_calculation_time,
                conflict,
                new_trip_events,
                prior_trip_events,
            }),
        ))
    } else {
        Ok((
            TripProcessingOutcome {
                num_trips: 0,
                state,
            },
            None,
        ))
    }
}

async fn new_vessel_events(
    vessel_id: FiskeridirVesselId,
    adapter: &dyn TripAssemblerOutboundPort,
    trip_assembler: TripAssemblerId,
    search_timestamp: TripSearchTimestamp,
) -> Result<(Vec<VesselEventDetailed>, Vec<VesselEventDetailed>)> {
    let prior_trip = adapter
        .trip_prior_to_timestamp(vessel_id, search_timestamp, trip_assembler)
        .await?;

    if let Some(prior_trip) = prior_trip {
        Ok((
            prior_trip.start_and_end_event.to_vec(),
            prior_trip.succeeding_events,
        ))
    } else {
        // Cases:
        // - We have a conflict that occurred prior to all other existing events of the vessel or
        // within the earliest trip. We should then retrieve all events and re-do all trips of the given vessel.
        // Cases that fit, but are handled elsewhere:
        // - First time processing a vessel no prior trip will exist, but this is handled by `AssemblerState::NoPriorState`.
        // - When a queued reset is set, but this is handled by `AssemblerState::QueuedReset `.
        let succeeding_events = adapter.all_vessel_events(vessel_id, trip_assembler).await?;
        Ok((vec![], succeeding_events))
    }
}

async fn process_unprocessed_trips(
    shared_state: &SharedState,
    vessel: &Vessel,
    ports: &HashMap<String, Port>,
    dock_points: &HashMap<String, Vec<PortDockPoint>>,
) -> Result<Vec<TripUpdate>> {
    let mut trips = HashMap::new();

    for (i, step) in TRIP_COMPUTATION_STEPS.iter().enumerate() {
        for trip in step
            .fetch_missing(shared_state, vessel, UNPROCESSED_TRIPS_BATCH_SIZE)
            .await?
        {
            trips
                .entry(trip.trip_id)
                .and_modify(|(_, idx)| *idx = min(*idx, i))
                .or_insert((trip, i));
        }
    }

    let mut updates = Vec::with_capacity(trips.len());

    for (t, computation_step_idx) in trips.into_values() {
        let mut unit = TripProcessingUnit {
            precision_outcome: None,
            distance_output: None,
            start_port: t
                .start_port_code
                .as_ref()
                .and_then(|v| ports.get(v).cloned()),
            end_port: t.end_port_code.as_ref().and_then(|v| ports.get(v).cloned()),
            start_dock_points: t
                .start_port_code
                .as_ref()
                .and_then(|v| dock_points.get(v).cloned())
                .unwrap_or_default(),
            end_dock_points: t
                .end_port_code
                .as_ref()
                .and_then(|v| dock_points.get(v).cloned())
                .unwrap_or_default(),
            positions: vec![],
            vessel_id: vessel.fiskeridir.id,
            trip_assembler_id: vessel.preferred_trip_assembler,
            trip: NewTrip {
                period: t.period.clone(),
                period_extended: t.period_extended.clone(),
                landing_coverage: t.landing_coverage.clone(),
                start_port_code: t.start_port_code.clone(),
                end_port_code: t.end_port_code.clone(),
                first_arrival: t.first_arrival,
                start_vessel_event_id: None,
                end_vessel_event_id: None,
            },
            position_layers_output: None,
            trip_id: t.trip_id,
        };

        TRIP_COMPUTATION_STEPS[computation_step_idx]
            .set_state(shared_state, &mut unit, vessel, &t)
            .await?;

        for step in &TRIP_COMPUTATION_STEPS[computation_step_idx..] {
            unit = step.run(shared_state, vessel, unit).await?;
        }

        updates.push(TripUpdate {
            trip_id: t.trip_id,
            precision: unit.precision_outcome,
            distance: unit.distance_output,
            positions: unit.positions,
            position_layers_output: unit.position_layers_output,
        });
    }

    Ok(updates)
}
