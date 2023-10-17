use std::collections::HashSet;

use crate::{
    error::PostgresError,
    models::{NewVmsPosition, VmsPosition},
    PostgresAdapter,
};
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::DateRange;
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn all_vms_impl(&self) -> Result<Vec<VmsPosition>, PostgresError> {
        sqlx::query_as!(
            VmsPosition,
            r#"
SELECT
    call_sign,
    course,
    latitude,
    longitude,
    registration_id,
    speed,
    "timestamp",
    vessel_length,
    vessel_name,
    vessel_type,
    distance_to_shore
FROM
    vms_positions
ORDER BY
    "timestamp" ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) fn vms_positions_impl(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> impl Stream<Item = Result<VmsPosition, PostgresError>> + '_ {
        sqlx::query_as!(
            VmsPosition,
            r#"
SELECT
    call_sign,
    course,
    latitude,
    longitude,
    registration_id,
    speed,
    "timestamp",
    vessel_length,
    vessel_name,
    vessel_type,
    distance_to_shore
FROM
    vms_positions
WHERE
    call_sign = $1
    AND "timestamp" BETWEEN $2 AND $3
ORDER BY
    "timestamp" ASC
            "#,
            call_sign.as_ref(),
            range.start(),
            range.end(),
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn add_vms_impl(
        &self,
        vms: Vec<fiskeridir_rs::Vms>,
    ) -> Result<(), PostgresError> {
        let mut call_signs_unique = HashSet::new();

        let vms = vms
            .into_iter()
            .filter(|v| v.latitude.is_some() && v.longitude.is_some())
            .map(NewVmsPosition::try_from)
            .inspect(|v| {
                if let Ok(ref v) = v {
                    call_signs_unique.insert(v.call_sign.clone());
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let call_signs_unique = call_signs_unique.into_iter().collect::<Vec<_>>();

        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
SELECT
    add_vms_position_partitions ($1)
            "#,
            call_signs_unique.as_slice(),
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

        NewVmsPosition::unnest_insert(vms, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
}
