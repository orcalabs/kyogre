use crate::models::AisVmsAreaPositionsReturning;
use std::collections::{HashMap, HashSet};

use crate::{
    error::Result,
    models::{EarliestVms, NewVmsPosition, VmsPosition},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use futures::{Stream, TryStreamExt};
use kyogre_core::{DateRange, Mmsi, PositionType};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn all_vms_impl(&self) -> Result<Vec<VmsPosition>> {
        let vms = sqlx::query_as!(
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
        .map_err(From::from)
    }

    pub(crate) async fn add_vms_impl(&self, vms: Vec<fiskeridir_rs::Vms>) -> Result<()> {
        let mut call_signs_unique = HashSet::new();
        let mut vms_unique: HashMap<(String, DateTime<Utc>), NewVmsPosition> = HashMap::new();
        let mut vms_earliest: HashMap<String, EarliestVms> = HashMap::new();

        let speed_threshold = 0.001;
        for v in vms {
            if v.latitude.is_none() || v.longitude.is_none() {
                continue;
            }

            let pos = NewVmsPosition::try_from(v)?;
            vms_earliest
                .entry(pos.call_sign.clone())
                .and_modify(|e| {
                    if e.timestamp > pos.timestamp {
                        e.timestamp = pos.timestamp;
                    }
                })
                .or_insert(EarliestVms {
                    call_sign: pos.call_sign.clone(),
                    timestamp: pos.timestamp,
                });

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
        let earliest_positions = vms_earliest.into_values().collect::<Vec<_>>();

        let mut tx = self.pool.begin().await?;
        EarliestVms::unnest_insert(earliest_positions, &mut *tx).await?;

        sqlx::query!(
            r#"
SELECT
    add_vms_position_partitions ($1)
            "#,
            call_signs_unique.as_slice(),
        )
        .execute(&mut *tx)
        .await?;

        let vms: Vec<NewVmsPosition> = vms_unique.into_values().collect();

        let len = vms.len();
        let mut lat = Vec::with_capacity(len);
        let mut lon = Vec::with_capacity(len);
        let mut timestamp = Vec::with_capacity(len);
        let mut position_type_id = Vec::with_capacity(len);
        let mut call_sign = Vec::with_capacity(len);
        for v in &vms {
            lat.push(v.latitude);
            lon.push(v.longitude);
            timestamp.push(v.timestamp);
            position_type_id.push(PositionType::Vms as i32);
            call_sign.push(v.call_sign.clone());
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
            &call_sign
        )
        .fetch_all(&mut *tx)
        .await?;

        self.add_ais_vms_aggregated(area_positions_inserted, &mut tx)
            .await?;

        NewVmsPosition::unnest_insert(vms, &mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
}
