use crate::{
    CatchValuePerFuel, Eeoi, FuelConsumption, Result, WeightPerDistance, WeightPerFuel,
    WeightPerHour,
};
use kyogre_core::{
    BenchmarkTrip, FiskeridirVesselId, ProcessingStatus, TripBenchmark, TripBenchmarkOutbound,
    TripBenchmarkOutput, TripId,
};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::task::JoinSet;
use tracing::error;

static RUN_INTERVAL: Duration = Duration::from_secs(1);
static NUM_WORKERS: usize = 8;

#[derive(Clone)]
pub struct TripBenchmarkRunner {
    adapter: Arc<dyn TripBenchmarkOutbound>,
    local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
    // All trips that have been sent to workers and are currently being processed.
    // Some trips might take longer to process than our `RUN_INTERVAL` and we use this set to avoid
    // sending the same trip to workers multiple times.
    in_progress: HashSet<TripId>,
}

struct Worker {
    adapter: Arc<dyn TripBenchmarkOutbound>,
    benchmarks: Vec<Box<dyn TripBenchmark>>,
    receiver: async_channel::Receiver<BenchmarkTrip>,
}

impl TripBenchmarkRunner {
    pub fn new(
        adapter: Arc<dyn TripBenchmarkOutbound>,
        local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
    ) -> Self {
        Self {
            adapter,
            local_processing_vessels,
            in_progress: HashSet::with_capacity(NUM_WORKERS),
        }
    }

    pub async fn run_continuous(mut self) -> ! {
        let mut task_set = JoinSet::new();
        let (sender, receiver) = async_channel::bounded(300);

        for _ in 0..NUM_WORKERS {
            let worker = Worker {
                adapter: self.adapter.clone(),
                benchmarks: enabled_benchmarks(),
                receiver: receiver.clone(),
            };
            task_set.spawn(worker.run());
        }

        task_set.spawn(async move {
            loop {
                self.run_cycle(&sender).await;
                tokio::time::sleep(RUN_INTERVAL).await;
            }
        });

        task_set.join_next().await.unwrap().unwrap();
    }

    async fn run_cycle(&mut self, sender: &async_channel::Sender<BenchmarkTrip>) {
        if let Err(e) = self.run_continuous_cycle(sender).await {
            let span = tracing::info_span!("benchmark_runner_cycle");
            let _guard = span.enter();
            error!("failed to run benchmarking cycle, err: {e:?}");
        }
    }

    async fn run_continuous_cycle(
        &mut self,
        sender: &async_channel::Sender<BenchmarkTrip>,
    ) -> Result<()> {
        if !sender.is_empty() {
            // There is still work available and there is no need to fetch more
            return Ok(());
        }
        let mut trips = self.adapter.trips_to_benchmark().await?;
        if let Some(vessel_ids) = &self.local_processing_vessels {
            trips.retain(|v| vessel_ids.contains(&v.vessel_id));
        }

        let trip_ids_to_benchmark = trips.iter().map(|t| t.trip_id).collect::<HashSet<TripId>>();

        // If our `in_progress` set includes a trip_id that no longer exists in `trips_to_benchmark` that means
        // the trip has successfully been processed and we can remove it from our `in_progress`
        // set.
        self.in_progress
            .retain(|t| trip_ids_to_benchmark.contains(t));

        for trip in trips {
            self.in_progress.insert(trip.trip_id);
            // SAFETY: Only returns an error if the channel is closed which means all workers have
            // exited unexpectedly and there is no way to recover.
            sender.send(trip).await.unwrap();
        }

        Ok(())
    }

    pub async fn run_single(&mut self) -> Result<()> {
        let mut trips = self.adapter.trips_to_benchmark().await?;
        if let Some(vessel_ids) = &self.local_processing_vessels {
            trips.retain(|v| vessel_ids.contains(&v.vessel_id));
        }

        for trip in trips {
            Worker::run_impl(&trip, self.adapter.as_ref(), &enabled_benchmarks()).await?;
        }

        Ok(())
    }
}

impl Worker {
    async fn run(self) -> ! {
        loop {
            // SAFETY: Only returns an error if the channel is closed which means the sender task
            // has exited unexpectedly and there is no way to recover.
            let trip = self.receiver.recv().await.unwrap();
            self.run_cycle(trip).await;
        }
    }

    async fn run_cycle(&self, trip: BenchmarkTrip) {
        if let Err(e) = Self::run_impl(&trip, self.adapter.as_ref(), &self.benchmarks).await {
            let span = tracing::info_span!("worker_run_cycle");
            let _guard = span.enter();
            error!(
                "failed to run benchmarks for trip '{}', err: {e:?}",
                trip.trip_id
            );
        }
    }

    async fn run_impl(
        trip: &BenchmarkTrip,
        adapter: &dyn TripBenchmarkOutbound,
        benchmarks: &[Box<dyn TripBenchmark>],
    ) -> Result<()> {
        let mut output = TripBenchmarkOutput {
            trip_id: trip.trip_id,
            weight_per_hour: None,
            weight_per_distance: None,
            fuel_consumption_liter: None,
            weight_per_fuel_liter: None,
            catch_value_per_fuel_liter: None,
            eeoi: None,
            status: ProcessingStatus::Successful,
            benchmark_state_counter: trip.benchmark_state_counter,
        };

        for b in benchmarks {
            b.benchmark(trip, adapter, &mut output).await?;
        }

        adapter.add_output(output).await?;

        Ok(())
    }
}

fn enabled_benchmarks() -> Vec<Box<dyn TripBenchmark>> {
    // Order is significant as some benchmarks depends on the output of others.
    // Currently most benchmarks depends on 'FuelConsumption'.
    vec![
        Box::<FuelConsumption>::default(),
        Box::<WeightPerHour>::default(),
        Box::<WeightPerDistance>::default(),
        Box::<WeightPerFuel>::default(),
        Box::<CatchValuePerFuel>::default(),
        Box::<Eeoi>::default(),
        // `Sustainability` needs to be last because it depends on benchmarks above.
        // TODO
        // Box::<Sustainability>::default(),
    ]
}
