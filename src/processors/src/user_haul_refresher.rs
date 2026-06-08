use crate::Result;
use kyogre_core::UserHaulsRefresher;
use std::{sync::Arc, time::Duration};
use tracing::{error, instrument};

static RUN_INTERVAL: Duration = Duration::from_secs(10);

/// The scheme for keeping user_hauls mappings to trips/hauls and keeping trips/current trip updated
/// is as follows (hauls refers to ERS hauls):
/// - All modification endpoints for user_hauls updates the 'user_haul_refresh_boundary' with the
///   lowest timestamp of the modification.
/// - When scraping ERS DCA we update the 'user_haul_refresh_boundary' with the lowest timestamp for
///   each vessel.
/// - This processor continuously checks the 'user_haul_refresh_boundary' and recreates the mapping
///   to trips/hauls for all user_hauls that ended after the boundary. User hauls are mapped to hauls
///   if they overlap with *ONLY* a single haul. User hauls are mapped to trips if they are not mapped
///   to a haul and they overlap with a *SINGLE* trip. This processor also refreshes the current
///   trip if any of the 'end_ts' of the user_hauls that had their mappings altered were *AFTER* the
///   start of the current trip. Also, during the user_hauls mappings refresh the trips refresh
///   boundary is updated with the *LOWEST* 'start_ts' of any of the user_hauls that had their
///   mappings altered. This is to ensure that trips will eventually get up to date hauls.
/// - When creating new trips we attach them to any user_hauls that are overlapping and according to
///   the rules mentioned above.
///
///   The user scenario when registering user_hauls will be as follows (we assume registering of new
///   user_hauls occurs in a live setting):
///   - Current trip will be updated with the new user_hauls by this processor within a short window
///     (10s)
///   - Updating/deleting an older user_haul will update its associated trip on
///     the next engine cycle (within the next 24H ish)
///   - Hauls to user_hauls mappings will be updated by this processor within a short window
///     (10s). This is currently not relevant as this mapping is only seen in the 'trips_detailed'
///     endpoint for now.
///
///   The user_hauls mappings will be modified concurrently by this processor and the trips state
///   when creating new trips. If this ends for some reason in a deadlock in either process this is
///   of little concern as both operations will simply be retried and no API user will see any errors.
///
///   The user_haul_refresh_boundary will be read by the user_hauls API endpoints and written by
///   this processor. Worst case is that the same rows are locked by both parties, resulting in a
///   longer wait time for the user_hauls requests which will affect the user experience.
///
///   Refreshing of the current trip is controlled by the current_trip_refresh_boundary column in the
///   user_haul_refresh_boundary table and should always be set to the *LATEST* modified start_ts of
///   any user_haul. A current_trip only contains hauls that have a start_ts equal to or greater
///   than its start. When refreshing the current trip we only update the hauls and we assume that
///   the trip assembler both creates the initial current trip and updates its other fields.
///   The scraper has to scrape the DEP messages to create the current trip and the Trips state runs
///   after scrape, so the delay from scrape to the creation of the current trip is not that long
///   (and we could not create the current trip anyway as we lack the DEP message).

#[derive(Clone)]
pub struct UserHaulRefresher {
    adapter: Arc<dyn UserHaulsRefresher>,
}

impl UserHaulRefresher {
    pub fn new(adapter: Arc<dyn UserHaulsRefresher>) -> Self {
        Self { adapter }
    }

    pub async fn run_continuous(self) -> ! {
        loop {
            self.run_cycle().await;
            tokio::time::sleep(RUN_INTERVAL).await;
        }
    }

    #[instrument(skip_all)]
    async fn run_cycle(&self) {
        if let Err(e) = self.run_single().await {
            error!("user haul refresher failed: {e:?}");
        }
    }

    pub async fn run_single(&self) -> Result<()> {
        self.adapter.refresh_user_haul_mappings().await?;

        Ok(())
    }
}
