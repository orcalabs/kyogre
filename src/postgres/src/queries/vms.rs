use crate::{
    PostgresAdapter,
    error::Result,
    models::{EarliestVms, NewVmsCurrentPosition, NewVmsPosition, VmsPosition},
};
use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{DateRange, EarliestVmsUsedBy, ProcessingStatus};
use std::collections::{HashMap, HashSet, hash_map::Entry};
use strum::IntoEnumIterator;

impl PostgresAdapter {
    pub(crate) fn vms_positions_impl<'a>(
        &'a self,
        call_sign: &'a CallSign,
        range: &'a DateRange,
    ) -> impl Stream<Item = Result<VmsPosition>> + 'a {
        self.vms_positions_inner(VmsPositionsArg::Filter { call_sign, range })
    }

    pub(crate) fn vms_positions_inner(
        &self,
        arg: VmsPositionsArg<'_>,
    ) -> impl Stream<Item = Result<VmsPosition>> + '_ {
        let (call_sign, start, end) = match arg {
            #[cfg(feature = "test")]
            VmsPositionsArg::All => (None, None, None),
            VmsPositionsArg::Filter { call_sign, range } => {
                (Some(call_sign), Some(range.start()), Some(range.end()))
            }
        };

        sqlx::query_as!(
            VmsPosition,
            r#"
SELECT
    call_sign AS "call_sign!: CallSign",
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
    (
        $1::TEXT IS NULL
        OR call_sign = $1
    )
    AND (
        $2::TIMESTAMPTZ IS NULL
        OR timestamp >= $2
    )
    AND (
        $3::TIMESTAMPTZ IS NULL
        OR timestamp <= $3
    )
ORDER BY
    "timestamp" ASC
            "#,
            call_sign as Option<&CallSign>,
            start,
            end,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn add_vms_impl(&self, vms: Vec<fiskeridir_rs::Vms>) -> Result<()> {
        let mut call_signs_unique = HashSet::new();
        let mut vms_unique: HashMap<(&str, DateTime<Utc>), NewVmsPosition<'_>> = HashMap::new();
        let mut vms_earliest: HashMap<&str, DateTime<Utc>> = HashMap::new();
        let mut current_positions: HashMap<&str, NewVmsCurrentPosition<'_>> = HashMap::new();

        let speed_threshold = 0.001;
        for v in &vms {
            if v.latitude.is_none() || v.longitude.is_none() {
                continue;
            }

            let call_sign = v.call_sign.as_ref();

            vms_earliest
                .entry(call_sign)
                .and_modify(|e| {
                    if *e > v.timestamp {
                        *e = v.timestamp;
                    }
                })
                .or_insert(v.timestamp);

            call_signs_unique.insert(call_sign);

            match vms_unique.entry((call_sign, v.timestamp)) {
                Entry::Vacant(e) => {
                    e.insert(v.try_into()?);
                }
                Entry::Occupied(mut e) => {
                    let e = e.get_mut();
                    let mut replace = false;

                    match (e.course, v.course) {
                        (Some(_), None) | (None, None) => (),
                        (None, Some(_)) => replace = true,
                        (Some(c), Some(c2)) => {
                            if c == 0 && c2 != 0 {
                                replace = true;
                            }
                        }
                    }
                    match (&e.speed, &v.speed) {
                        (Some(_), None) | (None, None) => (),
                        (None, Some(_)) => replace = true,
                        (Some(c), Some(c2)) => {
                            if *c < speed_threshold && *c2 > speed_threshold {
                                replace = true;
                            }
                        }
                    }

                    if replace {
                        *e = v.try_into()?;
                    }
                }
            }

            match current_positions.entry(call_sign) {
                Entry::Vacant(e) => {
                    e.insert(v.try_into()?);
                }
                Entry::Occupied(mut e) => {
                    if e.get().timestamp < v.timestamp {
                        e.insert(v.try_into()?);
                    }
                }
            }
        }

        let call_signs_unique = call_signs_unique.into_iter().collect::<Vec<_>>();
        let vms_earliest = vms_earliest.into_iter().flat_map(|(call_sign, timestamp)| {
            EarliestVmsUsedBy::iter().map(move |used_by| EarliestVms {
                call_sign,
                timestamp,
                used_by,
            })
        });

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(vms_earliest, &mut *tx).await?;
        self.unnest_insert(current_positions.into_values(), &mut *tx)
            .await?;

        sqlx::query!(
            r#"
SELECT
    add_vms_position_partitions ($1)
            "#,
            &call_signs_unique as &[&str],
        )
        .execute(&mut *tx)
        .await?;

        let inserted = self
            .unnest_insert_returning(vms_unique.into_values(), &mut *tx)
            .map_ok(|v| (v.call_sign, v.timestamp.date_naive()))
            .try_collect::<HashSet<_>>()
            .await?;

        let (call_signs, dates): (Vec<String>, Vec<NaiveDate>) = inserted.into_iter().unzip();

        sqlx::query!(
            r#"
WITH
    to_update AS (
        SELECT
            UNNEST($1::TEXT[]) call_sign,
            UNNEST($2::DATE[]) date
    )
UPDATE fuel_estimates f
SET
    status = $3
FROM
    (
        SELECT
            w.fiskeridir_vessel_id,
            to_update.date
        FROM
            to_update
            INNER JOIN all_vessels w ON w.call_sign = to_update.call_sign
    ) q
WHERE
    q.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    AND f.date = q.date
            "#,
            &call_signs,
            &dates,
            ProcessingStatus::Unprocessed as i32,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

pub(crate) enum VmsPositionsArg<'a> {
    #[cfg(feature = "test")]
    All,
    Filter {
        call_sign: &'a CallSign,
        range: &'a DateRange,
    },
}
