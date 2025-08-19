use crate::*;
use async_trait::async_trait;
use machine::Schedule;
use orca_core::Environment;
use tracing::{error, info};

pub struct DailyWeatherState;

#[async_trait]
impl machine::State for DailyWeatherState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or("test".into())
            .try_into()
            .unwrap();

        // On a fresh Local deployment it takes too long to perform the initial DailyWeather,
        // so prune the dirty table to only dates with weather.
        if environment == Environment::Local
            && let Err(e) = shared_state
                .catch_location_weather
                .prune_dirty_dates()
                .await
        {
            error!("failed to prune dirty weather dates: {e:?}");
        }

        match shared_state
            .catch_location_weather
            .catch_locations_with_weather()
            .await
        {
            Err(e) => {
                error!("failed to fetch catch location with weather: {e:?}");
            }
            Ok(cls) => match shared_state.catch_location_weather.dirty_dates().await {
                Err(e) => {
                    error!("failed to fetch missing catch location weather: {e:?}");
                }
                Ok(dates) => {
                    for (i, d) in dates.into_iter().enumerate() {
                        if let Err(e) = shared_state
                            .catch_location_weather
                            .update_daily_weather(&cls, d)
                            .await
                        {
                            error!("failed to update catch location daily average weather: {e:?}");
                        }

                        if i % 10 == 0 {
                            info!("update weather for {i} dates");
                        }
                    }
                }
            },
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
