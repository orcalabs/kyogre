use crate::{FiskeridirVesselId, VesselBenchmarkId};

pub struct Benchmark {
    pub vessel_id : FiskeridirVesselId,
    pub benchmark_id: VesselBenchmarkId,
    pub output : Option<f64>,
}