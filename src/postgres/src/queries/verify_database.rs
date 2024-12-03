use futures::TryStreamExt;
use kyogre_core::ActiveVesselConflict;
use orca_core::Environment;
use tracing::error;

use crate::{
    error::{
        BuyerLocationsWithoutMappingSnafu, ConflictingVesselMappingsSnafu,
        DanglingVesselEventsSnafu, IncorrectHaulCatchesSnafu,
        IncorrectHaulsMatrixLivingWeightSnafu, IncorrectLandingMatrixLivingWeightSnafu,
        LandingsWithoutTripSnafu, Result,
    },
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) async fn verify_database_impl(&self) -> Result<()> {
        match self.dangling_vessel_events().await? {
            0 => Ok(()),
            num => DanglingVesselEventsSnafu { num }.fail(),
        }?;

        match self.hauls_with_incorrect_catches().await? {
            message_ids if message_ids.is_empty() => Ok(()),
            message_ids => IncorrectHaulCatchesSnafu { message_ids }.fail(),
        }?;

        match self.hauls_matrix_vs_ers_dca_living_weight().await? {
            0 => Ok(()),
            weight_diff => IncorrectHaulsMatrixLivingWeightSnafu { weight_diff }.fail(),
        }?;

        match self.landing_matrix_vs_landings_living_weight().await? {
            0 => Ok(()),
            weight_diff => IncorrectLandingMatrixLivingWeightSnafu { weight_diff }.fail(),
        }?;

        let conflicts: Vec<ActiveVesselConflict> =
            self.active_vessel_conflicts_impl().try_collect().await?;

        if !conflicts.is_empty() {
            // We do not return an error here as we want to test this case in our tests and as a
            // workaround we log the error here instead of the verify database step
            let error = ConflictingVesselMappingsSnafu { conflicts }.build();

            // Dont want to spam test logs with this error message
            if self.environment != Environment::Test {
                error!("found vessel conflicts: {error:?}");
            }
        }

        match self.landings_without_trip().await? {
            0 => Ok(()),
            num => LandingsWithoutTripSnafu { num }.fail(),
        }?;

        match self.buyer_locations_without_mapping().await? {
            0 => Ok(()),
            num => BuyerLocationsWithoutMappingSnafu { num }.fail(),
        }?;

        Ok(())
    }
}
