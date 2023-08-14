use error_stack::{report, Result};

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

        Ok(())
    }
}
