use crate::*;
use async_trait::async_trait;
use machine::Schedule;
use tracing::{event, Level};

pub struct CatchLocationWeatherState;

#[async_trait]
impl machine::State for CatchLocationWeatherState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        match shared_state
            .catch_location_weather
            .catch_locations_with_weather()
            .await
        {
            Err(e) => {
                event!(
                    Level::ERROR,
                    "failed to fetch  catch location with weather: {:?}",
                    e
                );
            }
            Ok(cls) => match shared_state.catch_location_weather.dirty_dates().await {
                Err(e) => {
                    event!(
                        Level::ERROR,
                        "failed to fetch missing catch location weather: {:?}",
                        e
                    );
                }
                Ok(dates) => {
                    for (i, d) in dates.into_iter().enumerate() {
                        if let Err(e) = shared_state
                            .catch_location_weather
                            .update_catch_locations_weather(&cls, d)
                            .await
                        {
                            event!(
                                Level::ERROR,
                                "failed to update catch location daily average weather: {:?}",
                                e
                            );
                        }

                        if i % 10 == 0 {
                            event!(Level::INFO, "update weather for {i} dates");
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
