use num_traits::FromPrimitive;
use std::collections::{HashMap, HashSet};

use crate::{
    error::PostgresError,
    models::{NewVmsPosition, VmsPosition},
    PostgresAdapter,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
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
        let mut vms_unique: HashMap<(String, DateTime<Utc>), NewVmsPosition> = HashMap::new();

        let speed_threshold = BigDecimal::from_f64(0.001).unwrap();
        for v in vms {
            if v.latitude.is_none() || v.longitude.is_none() {
                continue;
            }

            let pos = NewVmsPosition::try_from(v)?;
            call_signs_unique.insert(pos.call_sign.clone());
            vms_unique
                .entry((pos.call_sign.clone(), pos.timestamp))
                .and_modify(|e| {
                    let mut replace = false;

                    match (e.course, pos.course) {
                        (Some(_), None) | (None, None) => (),
                        (None, Some(_)) => replace = true,
                        (Some(c), Some(c2)) => {
                            if c == 0 && c2 != 0 {
                                replace = true;
                            }
                        }
                    }
                    match (&e.speed, &pos.speed) {
                        (Some(_), None) | (None, None) => (),
                        (None, Some(_)) => replace = true,
                        (Some(c), Some(c2)) => {
                            if *c < speed_threshold && *c2 > speed_threshold {
                                replace = true;
                            }
                        }
                    }

                    if replace {
                        *e = pos.clone();
                    }
                })
                .or_insert(pos);
        }

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

        let vms: Vec<NewVmsPosition> = vms_unique.into_values().collect();
        NewVmsPosition::unnest_insert(vms, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
}
