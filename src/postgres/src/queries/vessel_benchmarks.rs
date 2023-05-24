use crate::PostgresAdapter;
use bigdecimal::{BigDecimal, FromPrimitive};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::VesselBenchmarkOutput;

use crate::error::PostgresError;

impl PostgresAdapter {
    pub(crate) async fn add_benchmark_outputs(
        &self,
        values: Vec<VesselBenchmarkOutput>,
    ) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        let len = values.len();
        let mut fiskeridir_vessel_id = Vec::with_capacity(len);
        let mut vessel_benchmark_id = Vec::with_capacity(len);
        let mut output = Vec::with_capacity(len);

        for v in values {
            fiskeridir_vessel_id.push(v.vessel_id.0);
            vessel_benchmark_id.push(v.benchmark_id as i32);
            output.push(BigDecimal::from_f64(v.value).ok_or(PostgresError::DataConversion)?);
        }

        sqlx::query!(
            r#"
INSERT INTO
    vessel_benchmark_outputs (fiskeridir_vessel_id, vessel_benchmark_id, output)
SELECT
    *
FROM
    UNNEST($1::BIGINT[], $2::INT[], $3::DECIMAL[])
ON CONFLICT (fiskeridir_vessel_id, vessel_benchmark_id) DO
UPDATE
SET
    output = excluded.output
            "#,
            fiskeridir_vessel_id.as_slice(),
            vessel_benchmark_id.as_slice(),
            output.as_slice()
        )
        .execute(&mut tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
}
