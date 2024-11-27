use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kyogre_core::{
    AisVmsPositionWithHaul, CoreResult, PositionType, TripBenchmark, TripBenchmarkId,
    TripBenchmarkOutbound, TripBenchmarkOutput, UpdateTripPositionFuel, Vessel,
};

/// Computes fuel consumption for a trip in tonnes.
#[derive(Default)]
pub struct FuelConsumption {}

// Suggested value from MARU experiments
static HAUL_LOAD_FACTOR: f64 = 1.75;

pub struct FuelItem {
    pub speed: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub position_type_id: PositionType,
    pub is_inside_haul_and_active_gear: bool,
}

pub struct FuelEstimation<T> {
    pub total_tonnage: f64,
    pub per_point: Vec<T>,
}

struct State {
    kwh: f64,
    prev: FuelItem,
}

impl From<AisVmsPositionWithHaul> for FuelItem {
    fn from(value: AisVmsPositionWithHaul) -> Self {
        FuelItem {
            speed: value.speed,
            timestamp: value.timestamp,
            position_type_id: value.position_type_id,
            is_inside_haul_and_active_gear: value.is_inside_haul_and_active_gear,
        }
    }
}

#[async_trait]
impl TripBenchmark for FuelConsumption {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::FuelConsumption
    }

    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        let Some(sfc) = vessel.sfc() else {
            return Ok(vec![]);
        };
        let Some(engine_power_kw) = vessel.engine_power_kw() else {
            return Ok(vec![]);
        };

        let trips = adapter
            .trips_without_fuel_consumption(vessel.fiskeridir.id)
            .await?;

        let mut output = Vec::with_capacity(trips.len());
        let mut fuel_updates = Vec::with_capacity(trips.len());

        for id in trips {
            let track = adapter.track_of_trip_with_haul(id).await?;

            if track.len() < 2 {
                continue;
            }

            let fuel_consumption_tonnes = estimate_fuel(
                sfc,
                engine_power_kw,
                track,
                &mut fuel_updates,
                |p, cumulative_fuel| UpdateTripPositionFuel {
                    trip_id: id,
                    timestamp: p.timestamp,
                    position_type_id: p.position_type_id,
                    trip_cumulative_fuel_consumption: cumulative_fuel,
                },
            );

            adapter
                .update_trip_position_fuel_consumption(&fuel_updates)
                .await?;
            fuel_updates.clear();

            output.push(TripBenchmarkOutput {
                trip_id: id,
                benchmark_id: TripBenchmarkId::FuelConsumption,
                value: fuel_consumption_tonnes,
                unrealistic: false,
            });
        }

        Ok(output)
    }
}

pub fn estimate_fuel<S, T, R>(
    sfc: f64,
    engine_power_kw: f64,
    items: Vec<R>,
    per_point: &mut Vec<S>,
    per_point_closure: T,
) -> f64
where
    T: Fn(&FuelItem, f64) -> S,
    R: Into<FuelItem>,
{
    if items.len() < 2 {
        return 0.0;
    }

    let mut iter = items.into_iter().map(R::into);

    let state = State {
        kwh: 0.,
        // `unwrap` is safe due to `len() < 2` check above
        prev: iter.next().unwrap(),
    };

    let result = iter.fold(state, |mut state, v| {
        let speed = match (state.prev.speed, v.speed) {
            (Some(a), Some(b)) => (a + b) / 2.,
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return state,
        };

        // TODO: Currently using surrogate value from:
        // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
        // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
        let speed_service = 12.;

        let load_factor = ((speed / speed_service).powf(3.) * 0.85).clamp(0., 0.98);

        state.kwh += load_factor
            * (engine_power_kw
                * if v.is_inside_haul_and_active_gear {
                    HAUL_LOAD_FACTOR
                } else {
                    1.0
                })
            * (v.timestamp - state.prev.timestamp).num_milliseconds() as f64
            / 3_600_000.;

        per_point.push(per_point_closure(&v, sfc * state.kwh / 1_000_000.));

        state.prev = v;
        state
    });

    sfc * result.kwh / 1_000_000.
}
