use std::{cmp::min, collections::HashMap, sync::Arc};

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use machine::Schedule;
use once_cell::sync::Lazy;
use tokio::sync::{mpsc::channel, Mutex};
use tracing::{event, Level};

static TRIP_COMPUTATION_STEPS: Lazy<Vec<Box<dyn TripComputationStep>>> = Lazy::new(|| {
    vec![
        Box::<TripPrecisionStep>::default(),
        Box::<TripPositionLayers>::default(),
        Box::<AisVms>::default(),
    ]
});

pub struct TripsState;

#[derive(Debug)]
struct VesselEvents {
    prior_trip_events: Vec<VesselEventDetailed>,
    new_vessel_events: Vec<VesselEventDetailed>,
}

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
    pub new_trip_calculation_time: DateTime<Utc>,
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
            AssemblerState::Normal(_) => (),
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
            Err(e) => event!(Level::ERROR, "failed to run trips pipeline: {:?}", e),
            Ok(r) => {
                event!(
                    Level::INFO,
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
                event!(
                    Level::ERROR,
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

#[async_trait]
trait TripComputationStep: Send + Sync {
    async fn run(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit, TripPipelineError>;
    async fn fetch_missing(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
    ) -> Result<Vec<Trip>, TripPipelineError>;
}

#[async_trait]
impl TripComputationStep for AisVms {
    async fn run(
        &self,
        _shared: &SharedState,
        _vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit, TripPipelineError> {
        unit.distance_output = Some(
            self.calculate_trip_distance(&unit)
                .change_context(TripPipelineError::TripComputationStep)?,
        );
        Ok(unit)
    }
    async fn fetch_missing(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
    ) -> Result<Vec<Trip>, TripPipelineError> {
        shared
            .trip_pipeline_outbound
            .trips_without_distance(vessel.fiskeridir.id)
            .await
            .change_context(TripPipelineError::TripComputationStep)
    }
}

#[derive(Default)]
struct TripPositionLayers;

#[async_trait]
impl TripComputationStep for TripPositionLayers {
    async fn run(
        &self,
        shared: &SharedState,
        _vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit, TripPipelineError> {
        let mut trip_positions = unit.positions;
        let mut pruned_positions = Vec::new();

        for l in &shared.trip_position_layers {
            let (positions, pruned) = l
                .prune_positions(trip_positions)
                .change_context(TripPipelineError::TripComputationStep)?;
            trip_positions = positions;
            pruned_positions.extend(pruned);
        }

        unit.positions = trip_positions.clone();
        unit.trip_position_output = Some(TripPositionLayerOutput {
            trip_positions,
            pruned_positions,
        });

        Ok(unit)
    }
    async fn fetch_missing(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
    ) -> Result<Vec<Trip>, TripPipelineError> {
        shared
            .trip_pipeline_outbound
            .trips_without_position_layers(vessel.fiskeridir.id)
            .await
            .change_context(TripPipelineError::TripComputationStep)
    }
}

#[derive(Default)]
struct TripPrecisionStep {
    landing: LandingTripAssembler,
    ers: ErsTripAssembler,
}

#[async_trait]
impl TripComputationStep for TripPrecisionStep {
    async fn run(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
        mut unit: TripProcessingUnit,
    ) -> Result<TripProcessingUnit, TripPipelineError> {
        if vessel.mmsi().is_none() && vessel.fiskeridir.call_sign.is_none() {
            return Ok(unit);
        }

        let adapter = shared.trips_precision_outbound_port.as_ref();
        let precision = match vessel.preferred_trip_assembler {
            TripAssemblerId::Landings => self.landing.calculate_precision(vessel, adapter, &unit),
            TripAssemblerId::Ers => self.ers.calculate_precision(vessel, adapter, &unit),
        }
        .await
        .change_context(TripPipelineError::TripComputationStep)?;

        unit.precision_outcome = Some(precision);

        if let Some(period_precison) = unit.precision_period() {
            unit.positions = shared
                .trips_precision_outbound_port
                .ais_vms_positions(
                    vessel.mmsi(),
                    vessel.fiskeridir.call_sign.as_ref(),
                    &period_precison,
                )
                .await
                .change_context(TripPipelineError::TripComputationStep)?;
        }

        Ok(unit)
    }
    async fn fetch_missing(
        &self,
        shared: &SharedState,
        vessel: &Vessel,
    ) -> Result<Vec<Trip>, TripPipelineError> {
        shared
            .trip_pipeline_outbound
            .trips_without_precision(vessel.fiskeridir.id)
            .await
            .change_context(TripPipelineError::TripComputationStep)
    }
}

enum MasterTask {
    New(
        Vessel,
        Result<(TripProcessingOutcome, Option<TripSet>), TripPipelineError>,
    ),
    Unprocessed(Vessel, Result<Vec<TripUpdate>, TripPipelineError>),
}

enum WorkerTask {
    New(Vessel),
    Unprocessed(Vessel),
    Done,
}

async fn run_state(shared_state: Arc<SharedState>) -> Result<TripsReport, TripPipelineError> {
    shared_state
        .trip_pipeline_inbound
        .update_preferred_trip_assemblers()
        .await
        .change_context(TripPipelineError::DataPreparation)?;

    shared_state
        .trip_pipeline_inbound
        .reset_trip_processing_conflicts()
        .await
        .change_context(TripPipelineError::DataPreparation)?;

    let vessels = shared_state
        .trip_assembler_outbound_port
        .all_vessels()
        .await
        .change_context(TripPipelineError::DataPreparation)?;

    if vessels.is_empty() {
        return Ok(Default::default());
    }

    let ports: HashMap<String, Port> = shared_state
        .trip_assembler_outbound_port
        .ports()
        .await
        .change_context(TripPipelineError::DataPreparation)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect::<HashMap<String, Port>>();

    let mut dock_points_map: HashMap<String, Vec<PortDockPoint>> = HashMap::new();
    let dock_points = shared_state
        .trip_assembler_outbound_port
        .dock_points()
        .await
        .change_context(TripPipelineError::DataPreparation)?;

    for d in dock_points {
        dock_points_map
            .entry(d.port_id.clone())
            .and_modify(|v| v.push(d.clone()))
            .or_insert(vec![d]);
    }

    let ports = Arc::new(ports);
    let dock_points = Arc::new(dock_points_map);

    let num_vessels = vessels.len();
    let num_workers = min(num_vessels, shared_state.num_workers as usize);

    let (master_tx, mut master_rx) = channel(10);
    let (worker_tx, worker_rx) = channel(num_vessels);
    let worker_rx = Arc::new(Mutex::new(worker_rx));

    let mut workers = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let master_tx = master_tx.clone();
        let worker_rx = worker_rx.clone();
        let shared_state = shared_state.clone();
        let ports = ports.clone();
        let dock_points = dock_points.clone();

        workers.push(tokio::spawn(async move {
            while let Some(task) = { worker_rx.lock().await.recv().await } {
                match task {
                    WorkerTask::New(vessel) => {
                        let result =
                            process_vessel(&shared_state, &vessel, &ports, &dock_points).await;
                        master_tx
                            .send(MasterTask::New(vessel, result))
                            .await
                            .unwrap()
                    }
                    WorkerTask::Unprocessed(vessel) => {
                        let result =
                            process_unprocessed_trips(&shared_state, &vessel, &ports, &dock_points)
                                .await;
                        master_tx
                            .send(MasterTask::Unprocessed(vessel, result))
                            .await
                            .unwrap()
                    }
                    WorkerTask::Done => return,
                }
            }
        }));
    }

    for v in vessels {
        worker_tx.try_send(WorkerTask::New(v)).unwrap();
    }

    let mut i = 0;
    let mut trips_report = TripsReport::default();

    while let Some(task) = master_rx.recv().await {
        match task {
            MasterTask::New(vessel, result) => {
                match result {
                    Ok((report, trips)) => {
                        trips_report = trips_report + report;

                        if let Some(trips) = trips {
                            if let Err(e) =
                                shared_state.trip_pipeline_inbound.add_trip_set(trips).await
                            {
                                event!(
                                    Level::ERROR,
                                    "failed to store trips for vessel: {}, err: {:?}",
                                    vessel.fiskeridir.id.0,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => event!(
                        Level::ERROR,
                        "failed to run trips pipeline for vessel: {}, err: {:?}",
                        vessel.fiskeridir.id.0,
                        e
                    ),
                }

                worker_tx.try_send(WorkerTask::Unprocessed(vessel)).unwrap();
            }
            MasterTask::Unprocessed(vessel, result) => {
                match result {
                    Ok(updates) => {
                        for update in updates {
                            let trip_id = update.trip_id;
                            if let Err(e) =
                                shared_state.trip_pipeline_inbound.update_trip(update).await
                            {
                                event!(
                                    Level::ERROR,
                                    "failed to update trip_id: {}, err: {:?}",
                                    trip_id.0,
                                    e
                                );
                            }
                        }

                        if let Err(e) = shared_state
                            .trip_pipeline_inbound
                            .refresh_detailed_trips(vessel.fiskeridir.id)
                            .await
                        {
                            event!(
                                Level::ERROR,
                                "failed to refresh detailed trips for vessel: {}, err: {:?}",
                                vessel.fiskeridir.id.0,
                                e
                            );
                        }
                    }
                    Err(e) => event!(
                        Level::ERROR,
                        "failed to process unprocessed trips for vessel: {}, err: {:?}",
                        vessel.fiskeridir.id.0,
                        e
                    ),
                }

                i += 1;
                if i % 1_000 == 0 {
                    event!(Level::INFO, "processed {}/{} vessels", i, num_vessels);
                }
                if i == num_vessels {
                    for _ in 0..num_workers {
                        worker_tx.try_send(WorkerTask::Done).unwrap();
                    }
                    break;
                }
            }
        }
    }

    for w in workers {
        w.await.unwrap();
    }

    Ok(trips_report)
}

async fn process_vessel(
    shared: &SharedState,
    vessel: &Vessel,
    ports: &HashMap<String, Port>,
    dock_points: &HashMap<String, Vec<PortDockPoint>>,
) -> Result<(TripProcessingOutcome, Option<TripSet>), TripPipelineError> {
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
            new_trip_calculation_time: trips.new_trip_calculation_time,
            trip_assembler_id: assembler_impl.assembler_id(),
            values: vec![],
            conflict: trips.conflict,
            new_trip_events: trips.new_trip_events,
            prior_trip_events: trips.prior_trip_events,
            prior_trip_calculation_time: trips.prior_trip_calculation_time,
        };
        for t in trips.trips {
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
                    .ais_vms_positions(
                        vessel.mmsi(),
                        vessel.fiskeridir.call_sign.as_ref(),
                        &t.period,
                    )
                    .await
                    .change_context(TripPipelineError::NewTripProcessing)?,
                vessel_id: vessel.fiskeridir.id,
                trip_assembler_id: output.trip_assembler_id,
                trip_position_output: None,
                trip: t,
            };

            for step in TRIP_COMPUTATION_STEPS.iter() {
                unit = step.run(shared, vessel, unit).await?;
            }

            output.values.push(unit);
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
) -> Result<(TripProcessingOutcome, Option<TripAssembly>), TripPipelineError> {
    let relevant_event_types = assembler.relevant_event_types();
    let timer = adapter
        .trip_calculation_timer(vessel.fiskeridir.id, assembler.assembler_id())
        .await
        .change_context(TripPipelineError::NewTripProcessing)?;

    let conflict = timer.as_ref().and_then(|v| v.conflict.clone());
    let prior_trip_calculation_time = timer.as_ref().map(|t| t.timestamp);

    let state = if let Some(timer) = timer {
        match (timer.conflict, timer.queued_reset) {
            (_, true) => AssemblerState::QueuedReset,
            (Some(c), false) => AssemblerState::Conflict(c),
            (None, false) => AssemblerState::Normal(timer.timestamp),
        }
    } else {
        AssemblerState::NoPriorState
    };

    let state_discriminant = AssemblerStateDiscriminants::from(&state);

    let vessel_events = match &state {
        AssemblerState::Conflict(c) => {
            new_vessel_events(
                vessel.fiskeridir.id,
                adapter,
                relevant_event_types,
                &c.timestamp,
                Bound::Exclusive,
            )
            .await
        }
        AssemblerState::Normal(t) => {
            new_vessel_events(
                vessel.fiskeridir.id,
                adapter,
                relevant_event_types,
                t,
                Bound::Inclusive,
            )
            .await
        }
        AssemblerState::NoPriorState | AssemblerState::QueuedReset => {
            all_vessel_events(vessel.fiskeridir.id, adapter, relevant_event_types).await
        }
    }?;

    let new_trip_events = vessel_events
        .new_vessel_events
        .iter()
        .map(MinimalVesselEvent::from)
        .collect();
    let prior_trip_events = vessel_events
        .prior_trip_events
        .iter()
        .map(MinimalVesselEvent::from)
        .collect();

    let trips = assembler
        .assemble(
            vessel_events.prior_trip_events,
            vessel_events.new_vessel_events,
        )
        .await
        .change_context(TripPipelineError::NewTripProcessing)?;

    if let Some(trips) = trips {
        let conflict_strategy = match (state_discriminant, trips.conflict_strategy) {
            (AssemblerStateDiscriminants::NoPriorState, Some(r))
            | (AssemblerStateDiscriminants::Normal, Some(r)) => r,
            (AssemblerStateDiscriminants::NoPriorState, None)
            | (AssemblerStateDiscriminants::Normal, None) => TripsConflictStrategy::Error,
            (AssemblerStateDiscriminants::Conflict, _) => TripsConflictStrategy::Replace,
            (AssemblerStateDiscriminants::QueuedReset, _) => TripsConflictStrategy::ReplaceAll,
        };

        Ok((
            TripProcessingOutcome {
                num_trips: trips.new_trips.len() as u32,
                state,
            },
            Some(TripAssembly {
                trips: trips.new_trips,
                conflict_strategy,
                new_trip_calculation_time: trips.calculation_timer,
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
    relevant_event_types: RelevantEventType,
    search_timestamp: &DateTime<Utc>,
    bound: Bound,
) -> Result<VesselEvents, TripPipelineError> {
    let prior_trip = adapter
        .trip_prior_to_timestamp(vessel_id, search_timestamp, bound)
        .await
        .change_context(TripPipelineError::NewTripProcessing)?;

    let res: Result<(Vec<VesselEventDetailed>, QueryRange), TripPipelineError> = match prior_trip {
        Some(prior_trip) => {
            let range = QueryRange::new(
                match prior_trip.period.end_bound() {
                    // We want all events not covered by the trip and therefore swap the bounds
                    crate::Bound::Inclusive => std::ops::Bound::Excluded(prior_trip.end()),
                    crate::Bound::Exclusive => std::ops::Bound::Included(prior_trip.end()),
                },
                std::ops::Bound::Unbounded,
            )
            .change_context(TripPipelineError::NewTripProcessing)?;

            let events = adapter
                .relevant_events(
                    vessel_id,
                    &QueryRange::from(prior_trip.period),
                    relevant_event_types,
                )
                .await
                .change_context(TripPipelineError::NewTripProcessing)?;

            Ok((events, range))
        }
        None => {
            let range = QueryRange::new(
                std::ops::Bound::Included(*search_timestamp),
                std::ops::Bound::Unbounded,
            )
            .change_context(TripPipelineError::NewTripProcessing)?;

            Ok((vec![], range))
        }
    };

    let (prior_trip_events, new_events_search_range) = res?;

    let new_vessel_events = adapter
        .relevant_events(vessel_id, &new_events_search_range, relevant_event_types)
        .await
        .change_context(TripPipelineError::NewTripProcessing)?;

    Ok(VesselEvents {
        prior_trip_events,
        new_vessel_events,
    })
}

async fn all_vessel_events(
    vessel_id: FiskeridirVesselId,
    adapter: &dyn TripAssemblerOutboundPort,
    relevant_event_types: RelevantEventType,
) -> Result<VesselEvents, TripPipelineError> {
    let range = QueryRange::new(std::ops::Bound::Unbounded, std::ops::Bound::Unbounded)
        .change_context(TripPipelineError::NewTripProcessing)?;

    let new_vessel_events = adapter
        .relevant_events(vessel_id, &range, relevant_event_types)
        .await
        .change_context(TripPipelineError::NewTripProcessing)?;

    Ok(VesselEvents {
        prior_trip_events: vec![],
        new_vessel_events,
    })
}

async fn process_unprocessed_trips(
    shared_state: &SharedState,
    vessel: &Vessel,
    ports: &HashMap<String, Port>,
    dock_points: &HashMap<String, Vec<PortDockPoint>>,
) -> Result<Vec<TripUpdate>, TripPipelineError> {
    let mut trips = HashMap::new();

    for (i, step) in TRIP_COMPUTATION_STEPS.iter().enumerate() {
        for trip in step.fetch_missing(shared_state, vessel).await? {
            trips
                .entry(trip.trip_id)
                .and_modify(|(_, idx)| *idx = min(*idx, i))
                .or_insert((trip, i));
        }
    }

    let mut updates = Vec::with_capacity(trips.len());

    for (t, idx) in trips.into_values() {
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
            positions: shared_state
                .trips_precision_outbound_port
                .ais_vms_positions(
                    vessel.mmsi(),
                    vessel.fiskeridir.call_sign.as_ref(),
                    &t.period,
                )
                .await
                .change_context(TripPipelineError::ExistingTripProcessing)?,
            vessel_id: vessel.fiskeridir.id,
            trip_assembler_id: vessel.preferred_trip_assembler,
            trip: NewTrip {
                period: t.period.clone(),
                landing_coverage: t.landing_coverage,
                start_port_code: t.start_port_code,
                end_port_code: t.end_port_code,
            },
            trip_position_output: None,
        };

        for step in &TRIP_COMPUTATION_STEPS[idx..] {
            unit = step.run(shared_state, vessel, unit).await?;
        }

        updates.push(TripUpdate {
            trip_id: t.trip_id,
            precision: unit.precision_outcome,
            distance: unit.distance_output,
            position_layers: unit.trip_position_output,
        });
    }

    Ok(updates)
}
