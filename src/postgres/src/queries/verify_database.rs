use error_stack::{report, Result};
use orca_core::Environment;
use tracing::{event, Level};

use crate::{
    error::{PostgresError, VerifyDatabaseError},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) async fn verify_database_impl(&self) -> Result<(), PostgresError> {
        match self.dangling_vessel_events().await? {
            0 => Ok(()),
            v => Err(report!(VerifyDatabaseError::DanglingVesselEvents(v))
                .change_context(PostgresError::InconsistentState)),
        }?;

        match self.hauls_with_incorrect_catches().await? {
            v if v.is_empty() => Ok(()),
            v => Err(report!(VerifyDatabaseError::IncorrectHaulCatches(v))
                .change_context(PostgresError::InconsistentState)),
        }?;

        match self.hauls_matrix_vs_ers_dca_living_weight().await? {
            0 => Ok(()),
            v => Err(
                report!(VerifyDatabaseError::IncorrectHaulsMatrixLivingWeight(v))
                    .change_context(PostgresError::InconsistentState),
            ),
        }?;

        match self.landing_matrix_vs_landings_living_weight().await? {
            0 => Ok(()),
            v => Err(
                report!(VerifyDatabaseError::IncorrectLandingMatrixLivingWeight(v))
                    .change_context(PostgresError::InconsistentState),
            ),
        }?;

        let conflicts = self.active_vessel_conflicts_impl().await?;
        if !conflicts.is_empty() {
            // We do not return an error here as we want to test this case in our tests and as a
            // workaround we log the error here instead of the verify database step
            let report = report!(VerifyDatabaseError::ConflictingVesselMappings(conflicts))
                .change_context(PostgresError::InconsistentState);

            // Dont want to spam test logs with this error message
            if let Some(env) = self.environment {
                if !matches!(env, Environment::Test) {
                    event!(Level::ERROR, "found vessel conflicts: {:?}", report);
                }
            } else {
                event!(Level::ERROR, "found vessel conflicts: {:?}", report);
            }
        }

        match self.landings_without_trip().await? {
            0 => Ok(()),
            v => Err(report!(VerifyDatabaseError::LandingsWithoutTrip(v))
                .change_context(PostgresError::InconsistentState)),
        }?;

        Ok(())
    }
}
