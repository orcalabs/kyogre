use crate::{models::{VesselBenchmarkOutput, Benchmarks}, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt, report};
use futures::{Stream, TryStreamExt};
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
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) fn benchmark_impl(&self) -> impl Stream<Item = Result<Benchmarks, PostgresError>> + '_ {
        sqlx::query_as!(
            Benchmarks,
            r#"
SELECT 
    fiskeridir_vessel_id as vessel_id,
	vessel_benchmark_id as benchmark_id,
	output
FROM
    vessel_benchmark_outputs
ORDER BY
    benchmark_id,
    output DESC
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }
}
