use std::collections::{hash_map::Entry, HashMap, HashSet};

use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{DateRange, Mmsi, PositionType};

use crate::{
    error::Result,
    models::{AisVmsAreaPositionsReturning, EarliestVms, NewVmsPosition, VmsPosition},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) async fn all_vms_impl(&self) -> Result<Vec<VmsPosition>> {
        let vms = sqlx::query_as!(
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
ORDER BY
    "timestamp" ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(vms)
    }

    pub(crate) fn vms_positions_impl(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> impl Stream<Item = Result<VmsPosition>> + '_ {
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
        .map_err(|e| e.into())
    }

    pub(crate) async fn add_vms_impl(&self, vms: Vec<fiskeridir_rs::Vms>) -> Result<()> {
        let mut call_signs_unique = HashSet::new();
        let mut vms_unique: HashMap<(&str, DateTime<Utc>), NewVmsPosition<'_>> = HashMap::new();
        let mut vms_earliest: HashMap<&str, EarliestVms<'_>> = HashMap::new();

        let speed_threshold = 0.001;
        for v in &vms {
            if v.latitude.is_none() || v.longitude.is_none() {
                continue;
            }

            let call_sign = v.call_sign.as_ref();

            vms_earliest
                .entry(call_sign)
                .and_modify(|e| {
                    if e.timestamp > v.timestamp {
                        e.timestamp = v.timestamp;
                    }
                })
                .or_insert(EarliestVms {
                    call_sign,
                    timestamp: v.timestamp,
                });

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
        }

        let call_signs_unique = call_signs_unique.into_iter().collect::<Vec<_>>();
        let earliest_positions = vms_earliest.into_values();

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(earliest_positions, &mut *tx).await?;

        sqlx::query!(
            r#"
SELECT
    add_vms_position_partitions ($1)
            "#,
            &call_signs_unique as &[&str],
        )
        .execute(&mut *tx)
        .await?;

        let len = vms_unique.len();
        let mut lat = Vec::with_capacity(len);
        let mut lon = Vec::with_capacity(len);
        let mut timestamp = Vec::with_capacity(len);
        let mut position_type_id = Vec::with_capacity(len);
        let mut call_sign = Vec::with_capacity(len);

        for v in vms_unique.values() {
            lat.push(v.latitude);
            lon.push(v.longitude);
            timestamp.push(v.timestamp);
            position_type_id.push(PositionType::Vms as i32);
            call_sign.push(v.call_sign);
        }

        let area_positions_inserted = sqlx::query_as!(
            AisVmsAreaPositionsReturning,
            r#"
INSERT INTO
    ais_vms_area_positions AS a (
        latitude,
        longitude,
        call_sign,
        "timestamp",
        position_type_id,
        mmsi
    )
SELECT
    u.latitude,
    u.longitude,
    u.call_sign,
    u."timestamp",
    u.position_type_id,
    NULL
FROM
    UNNEST(
        $1::DOUBLE PRECISION[],
        $2::DOUBLE PRECISION[],
        $3::TIMESTAMPTZ[],
        $4::INT[],
        $5::VARCHAR[]
    ) u (
        latitude,
        longitude,
        "timestamp",
        position_type_id,
        call_sign
    )
ON CONFLICT DO NOTHING
RETURNING
    a.latitude,
    a.longitude,
    a."timestamp",
    a.call_sign,
    a.mmsi AS "mmsi?: Mmsi"
            "#,
            &lat,
            &lon,
            &timestamp,
            &position_type_id,
            &call_sign as &[&str],
        )
        .fetch_all(&mut *tx)
        .await?;

        self.add_ais_vms_aggregated(area_positions_inserted, &mut tx)
            .await?;

        self.unnest_insert(vms_unique.into_values(), &mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}
