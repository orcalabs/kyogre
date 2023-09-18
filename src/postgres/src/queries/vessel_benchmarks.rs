use crate::{models::VesselBenchmarkOutput, PostgresAdapter};
use error_stack::{Result, ResultExt};
use unnest_insert::UnnestInsert;

use crate::error::PostgresError;

impl PostgresAdapter {
    pub(crate) async fn add_benchmark_outputs(
        &self,
        values: Vec<kyogre_core::VesselBenchmarkOutput>,
    ) -> Result<(), PostgresError> {
        let values = values
            .into_iter()
            .map(VesselBenchmarkOutput::try_from)
            .collect::<Result<_, _>>()?;

        VesselBenchmarkOutput::unnest_insert(values, &self.pool)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
