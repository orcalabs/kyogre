use crate::{test_helper::item_distribution::ItemDistribution, *};
use chrono::Duration;
use fiskeridir_rs::ErsDca;

pub struct HaulBuilder {
    pub state: TestStateBuilder,
    pub current_index: usize,
}

pub struct HaulTripBuilder {
    pub state: TripBuilder,
    pub current_index: usize,
}

pub struct HaulVesselBuilder {
    pub state: VesselBuilder,
    pub current_index: usize,
}

#[derive(Clone, Debug)]
pub struct HaulConstructor {
    pub dca: ErsDca,
}

impl HaulVesselBuilder {
    pub fn weather(mut self, amount: usize) -> WeatherHaulBuilder {
        assert_ne!(amount, 0);
        let lats_lons_times = {
            let base = &mut self.state.state;

            let hauls = &mut base.hauls[self.current_index..];
            let distribution = ItemDistribution::new(amount, hauls.len());

            let mut lats_lons_times = Vec::new();

            for (i, haul) in hauls.iter_mut().enumerate() {
                let num_weather = distribution.num_elements(i);

                let start = haul.dca.start_timestamp.unwrap();
                let diff = haul.dca.stop_timestamp.unwrap() - start;
                let increment = diff.num_milliseconds() / num_weather as i64;

                for i in 0..num_weather {
                    let weather = NewWeather::test_default(
                        start + Duration::milliseconds(increment * i as i64),
                    );
                    lats_lons_times.push((weather.latitude, weather.longitude, weather.timestamp));
                    base.weather.push(WeatherConstructor { weather });
                }
            }
            lats_lons_times
        };

        self.state = self
            .state
            .ais_positions(lats_lons_times.len())
            .modify_idx(|i, p| {
                p.position.latitude = lats_lons_times[i].0;
                p.position.longitude = lats_lons_times[i].1;
                p.position.msgtime = lats_lons_times[i].2;
            })
            .state;

        WeatherHaulBuilder {
            current_index: self.state.state.weather.len() - amount,
            state: self,
        }
    }
}
