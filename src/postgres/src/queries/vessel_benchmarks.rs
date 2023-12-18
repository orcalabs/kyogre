use crate::{error::PostgresErrorWrapper, models::VesselBenchmarkOutput, PostgresAdapter};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_benchmark_outputs(
        &self,
        values: Vec<kyogre_core::VesselBenchmarkOutput>,
    ) -> Result<(), PostgresErrorWrapper> {
        let values = values
            .into_iter()
            .map(VesselBenchmarkOutput::try_from)
            .collect::<Result<_, _>>()?;

        VesselBenchmarkOutput::unnest_insert(values, &self.pool).await?;

        Ok(())
    }
}
