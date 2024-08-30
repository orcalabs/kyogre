use orca_core::Environment;
use tracing::error;

use crate::{
    error::{PostgresErrorWrapper, VerifyDatabaseError},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) async fn verify_database_impl(&self) -> Result<(), PostgresErrorWrapper> {
        match self.dangling_vessel_events().await? {
            0 => Ok(()),
            v => Err(VerifyDatabaseError::DanglingVesselEvents(v)),
        }?;

        match self.hauls_with_incorrect_catches().await? {
            v if v.is_empty() => Ok(()),
            v => Err(VerifyDatabaseError::IncorrectHaulCatches(v)),
        }?;

        match self.hauls_matrix_vs_ers_dca_living_weight().await? {
            0 => Ok(()),
            v => Err(VerifyDatabaseError::IncorrectHaulsMatrixLivingWeight(v)),
        }?;

        match self.landing_matrix_vs_landings_living_weight().await? {
            0 => Ok(()),
            v => Err(VerifyDatabaseError::IncorrectLandingMatrixLivingWeight(v)),
        }?;

        let conflicts = self.active_vessel_conflicts_impl().await?;
        if !conflicts.is_empty() {
            // We do not return an error here as we want to test this case in our tests and as a
            // workaround we log the error here instead of the verify database step
            let error: PostgresErrorWrapper =
                VerifyDatabaseError::ConflictingVesselMappings(conflicts).into();

            // Dont want to spam test logs with this error message
            if self.environment != Environment::Test {
                error!("found vessel conflicts: {error:?}");
            }
        }

        match self.landings_without_trip().await? {
            0 => Ok(()),
            v => Err(VerifyDatabaseError::LandingsWithoutTrip(v)),
        }?;

        Ok(())
    }
}
