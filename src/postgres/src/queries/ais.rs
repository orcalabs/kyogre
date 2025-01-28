use std::collections::HashMap;

use chrono::{DateTime, Utc};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisPosition, AisVesselMigrate, DateRange, Mmsi, NavigationStatus,
    NewAisPosition, NewAisStatic, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES,
    PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};

use crate::{
    error::Result,
    models::{self, NewAisCurrentPosition, NewAisVessel, NewAisVesselHistoric, NewAisVesselMmsi},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn ais_positions_impl(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisPosition>> + '_ {
        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    mmsi AS "mmsi!: Mmsi",
    TIMESTAMP AS msgtime,
    course_over_ground,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    speed_over_ground,
    true_heading,
    distance_to_shore
FROM
    ais_positions
WHERE
    mmsi = $1
    AND TIMESTAMP BETWEEN $2 AND $3
    AND $1 IN (
        SELECT
            a.mmsi
        FROM
            ais_vessels a
            LEFT JOIN fiskeridir_vessels f ON a.call_sign = f.call_sign
        WHERE
            a.mmsi = $1
            AND (
                a.ship_type IS NOT NULL
                AND NOT (a.ship_type = ANY ($4::INT[]))
                OR COALESCE(f.length, a.ship_length) > $5
            )
            AND (
                CASE
                    WHEN $6 = 0 THEN TRUE
                    WHEN $6 = 1 THEN COALESCE(f.length, a.ship_length) >= $7
                END
            )
    )
ORDER BY
    TIMESTAMP ASC
            "#,
            mmsi.into_inner(),
            range.start(),
            range.end(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
        )
        .fetch(self.ais_pool())
        .map_err(|e| e.into())
    }

    pub(crate) async fn all_ais_positions_impl(
        &self,
        arg: AisPositionsArg,
    ) -> Result<Vec<AisPosition>> {
        let (mmsi, start, end) = match arg {
            #[cfg(feature = "test")]
            AisPositionsArg::All => (None, None, None),
            AisPositionsArg::Filter { mmsi, start, end } => (Some(mmsi), Some(start), Some(end)),
        };

        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    mmsi AS "mmsi!: Mmsi",
    timestamp AS msgtime,
    course_over_ground,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    speed_over_ground,
    true_heading,
    distance_to_shore
FROM
    ais_positions
WHERE
    (
        $1::INT IS NULL
        OR mmsi = $1
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
    timestamp ASC
            "#,
            mmsi as Option<Mmsi>,
            start,
            end,
        )
        .fetch_all(self.ais_pool())
        .await
        .map_err(|e| e.into())
    }

    pub(crate) async fn existing_mmsis_impl(&self) -> Result<Vec<Mmsi>> {
        let mmsis = sqlx::query!(
            r#"
SELECT
    mmsi AS "mmsi!: Mmsi"
FROM
    ais_vessels
            "#,
        )
        .fetch(&self.pool)
        .map_ok(|v| v.mmsi)
        .try_collect()
        .await?;

        Ok(mmsis)
    }

    pub(crate) async fn add_ais_positions(&self, positions: &[NewAisPosition]) -> Result<()> {
        let len = positions.len();
        let mut mmsis = Vec::<NewAisVesselMmsi>::with_capacity(len);
        let mut ais_positions = Vec::<models::NewAisPosition>::with_capacity(len);
        let mut current_positions = HashMap::<Mmsi, NewAisCurrentPosition>::with_capacity(len);

        for p in positions {
            mmsis.push(p.into());
            ais_positions.push(p.into());
            current_positions
                .entry(p.mmsi)
                .and_modify(|v| {
                    if p.msgtime > v.timestamp {
                        *v = p.into();
                    }
                })
                .or_insert_with(|| p.into());
        }

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(mmsis, &mut *tx).await?;
        self.unnest_insert(ais_positions, &mut *tx).await?;
        self.unnest_insert(current_positions.into_values(), &mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn ais_vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>> {
        Ok(sqlx::query_as!(
            AisVesselMigrate,
            r#"
SELECT
    mmsi AS "mmsi!: Mmsi",
    progress
FROM
    ais_data_migration_progress
WHERE
    progress < $1
            "#,
            migration_end_threshold
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn add_ais_vessels(&self, static_messages: &[NewAisStatic]) -> Result<()> {
        let mut unique_static: HashMap<Mmsi, NewAisVessel<'_>> = HashMap::new();
        for v in static_messages {
            unique_static.entry(v.mmsi).or_insert_with(|| v.into());
        }

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(unique_static.into_values(), &mut *tx)
            .await?;
        self.unnest_insert_from::<_, _, NewAisVesselHistoric<'_>>(static_messages, &mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_mmsis_impl(&self, mmsis: &[Mmsi]) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    ais_vessels (mmsi)
SELECT
    *
FROM
    UNNEST($1::INT[])
ON CONFLICT (mmsi) DO NOTHING
            "#,
            &mmsis as &[Mmsi],
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_ais_migration_data(
        &self,
        mmsi: Mmsi,
        positions: Vec<kyogre_core::AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<()> {
        let mut mmsis = Vec::with_capacity(positions.len());
        let mut latitude = Vec::with_capacity(positions.len());
        let mut longitude = Vec::with_capacity(positions.len());
        let mut course_over_ground = Vec::with_capacity(positions.len());
        let mut rate_of_turn = Vec::with_capacity(positions.len());
        let mut true_heading = Vec::with_capacity(positions.len());
        let mut speed_over_ground = Vec::with_capacity(positions.len());
        let mut timestamp = Vec::with_capacity(positions.len());
        let mut distance_to_shore = Vec::with_capacity(positions.len());
        let mut navigation_status_id = Vec::with_capacity(positions.len());

        for p in positions {
            mmsis.push(p.mmsi);
            latitude.push(p.latitude);
            longitude.push(p.longitude);
            course_over_ground.push(p.course_over_ground);
            rate_of_turn.push(p.rate_of_turn);
            true_heading.push(p.true_heading);
            speed_over_ground.push(p.speed_over_ground);
            distance_to_shore.push(p.distance_to_shore);
            navigation_status_id.push(p.navigational_status.map(|v| v as i32));
            timestamp.push(p.msgtime);
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    ais_data_migration_progress (mmsi, progress)
VALUES
    ($1, $2)
ON CONFLICT (mmsi) DO UPDATE
SET
    progress = excluded.progress
            "#,
            mmsi.into_inner(),
            &progress
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    ais_positions (
        mmsi,
        latitude,
        longitude,
        course_over_ground,
        rate_of_turn,
        true_heading,
        speed_over_ground,
        TIMESTAMP,
        distance_to_shore,
        navigation_status_id
    )
SELECT
    *
FROM
    UNNEST(
        $1::INT[],
        $2::DOUBLE PRECISION[],
        $3::DOUBLE PRECISION[],
        $4::DOUBLE PRECISION[],
        $5::DOUBLE PRECISION[],
        $6::INT[],
        $7::DOUBLE PRECISION[],
        $8::TIMESTAMPTZ[],
        $9::DOUBLE PRECISION[],
        $10::INT[]
    )
ON CONFLICT (mmsi, TIMESTAMP) DO NOTHING
            "#,
            &mmsis as &[Mmsi],
            &latitude,
            &longitude,
            &course_over_ground as _,
            &rate_of_turn as _,
            &true_heading as _,
            &speed_over_ground as _,
            &timestamp,
            &distance_to_shore,
            &navigation_status_id as _,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

pub(crate) enum AisPositionsArg {
    #[cfg(feature = "test")]
    All,
    Filter {
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}
