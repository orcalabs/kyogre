use std::collections::HashMap;

use chrono::{DateTime, Utc};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisPosition, AisVesselMigrate, DateRange, Mmsi, NavigationStatus,
    NewAisPosition, NewAisStatic, PositionType, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY,
    LEISURE_VESSEL_SHIP_TYPES, PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};

use crate::{
    error::Result,
    models::{AisVmsAreaPositionsReturning, NewAisVessel, NewAisVesselHistoric},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn ais_current_positions(
        &self,
        limit: Option<DateTime<Utc>>,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisPosition>> + '_ {
        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    c.mmsi AS "mmsi!: Mmsi",
    TIMESTAMP AS msgtime,
    course_over_ground,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    speed_over_ground,
    true_heading,
    distance_to_shore
FROM
    current_ais_positions c
    INNER JOIN ais_vessels a ON c.mmsi = a.mmsi
    LEFT JOIN fiskeridir_vessels f ON a.call_sign = f.call_sign
WHERE
    (
        $1::timestamptz IS NULL
        OR TIMESTAMP > $1
    )
    AND (
        a.ship_type IS NOT NULL
        AND NOT (a.ship_type = ANY ($2::INT[]))
        OR COALESCE(f.length, a.ship_length) > $3
    )
    AND (
        CASE
            WHEN $4 = 0 THEN TRUE
            WHEN $4 = 1 THEN COALESCE(f.length, a.ship_length) >= $5
        END
    )
            "#,
            limit,
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

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
        let mut mmsis = Vec::with_capacity(positions.len());
        let mut latitude = Vec::with_capacity(positions.len());
        let mut longitude = Vec::with_capacity(positions.len());
        let mut course_over_ground = Vec::with_capacity(positions.len());
        let mut rate_of_turn = Vec::with_capacity(positions.len());
        let mut true_heading = Vec::with_capacity(positions.len());
        let mut speed_over_ground = Vec::with_capacity(positions.len());
        let mut timestamp = Vec::with_capacity(positions.len());
        let mut altitude = Vec::with_capacity(positions.len());
        let mut distance_to_shore = Vec::with_capacity(positions.len());
        let mut navigation_status_id = Vec::with_capacity(positions.len());
        let mut ais_class = Vec::with_capacity(positions.len());
        let mut ais_message_type = Vec::with_capacity(positions.len());

        let mut latest_position_per_vessel: HashMap<Mmsi, NewAisPosition> = HashMap::new();

        for p in positions {
            latest_position_per_vessel
                .entry(p.mmsi)
                .and_modify(|v| {
                    if p.msgtime > v.msgtime {
                        *v = p.clone();
                    }
                })
                .or_insert_with(|| p.clone());

            mmsis.push(p.mmsi);
            latitude.push(p.latitude);
            longitude.push(p.longitude);
            course_over_ground.push(p.course_over_ground);
            rate_of_turn.push(p.rate_of_turn);
            true_heading.push(p.true_heading);
            speed_over_ground.push(p.speed_over_ground);
            altitude.push(p.altitude);
            distance_to_shore.push(p.distance_to_shore);
            navigation_status_id.push(p.navigational_status as i32);
            timestamp.push(p.msgtime);
            ais_class.push(p.ais_class.map(<&str>::from));
            ais_message_type.push(p.message_type_id);
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    ais_vessels (mmsi)
VALUES
    (UNNEST($1::INT[]))
ON CONFLICT (mmsi) DO NOTHING
            "#,
            &mmsis as &[Mmsi],
        )
        .execute(&mut *tx)
        .await?;

        let inserted = sqlx::query!(
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
        altitude,
        distance_to_shore,
        ais_class,
        ais_message_type_id,
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
        $9::INT[],
        $10::DOUBLE PRECISION[],
        $11::VARCHAR[],
        $12::INT[],
        $13::INT[]
    )
ON CONFLICT (mmsi, TIMESTAMP) DO NOTHING
RETURNING
    mmsi AS "mmsi!: Mmsi",
    latitude,
    longitude,
    "timestamp"
            "#,
            &mmsis as &[Mmsi],
            &latitude,
            &longitude,
            &course_over_ground as _,
            &rate_of_turn as _,
            &true_heading as _,
            &speed_over_ground as _,
            &timestamp,
            &altitude as _,
            &distance_to_shore,
            &ais_class as _,
            &ais_message_type as _,
            &navigation_status_id,
        )
        .fetch_all(&mut *tx)
        .await?;

        for (_, p) in latest_position_per_vessel {
            let latitude = p.latitude;
            let longitude = p.longitude;
            let course_over_ground = p.course_over_ground;
            let rate_of_turn = p.rate_of_turn;
            let speed_over_ground = p.speed_over_ground;
            let distance_to_shore = p.distance_to_shore;
            let ais_class = p.ais_class.map(<&str>::from);

            sqlx::query!(
                r#"
INSERT INTO
    current_ais_positions (
        mmsi,
        latitude,
        longitude,
        course_over_ground,
        rate_of_turn,
        true_heading,
        speed_over_ground,
        TIMESTAMP,
        altitude,
        distance_to_shore,
        ais_class,
        ais_message_type_id,
        navigation_status_id
    )
VALUES
    (
        $1::INT,
        $2::DOUBLE PRECISION,
        $3::DOUBLE PRECISION,
        $4::DOUBLE PRECISION,
        $5::DOUBLE PRECISION,
        $6::INT,
        $7::DOUBLE PRECISION,
        $8::timestamptz,
        $9::INT,
        $10::DOUBLE PRECISION,
        $11::VARCHAR,
        $12::INT,
        $13::INT
    )
ON CONFLICT (mmsi) DO
UPDATE
SET
    latitude = excluded.latitude,
    longitude = excluded.longitude,
    course_over_ground = excluded.course_over_ground,
    rate_of_turn = excluded.rate_of_turn,
    true_heading = excluded.true_heading,
    speed_over_ground = excluded.speed_over_ground,
    TIMESTAMP = excluded.timestamp,
    altitude = excluded.altitude,
    distance_to_shore = excluded.distance_to_shore,
    ais_class = excluded.ais_class,
    ais_message_type_id = excluded.ais_message_type_id,
    navigation_status_id = excluded.navigation_status_id
                "#,
                p.mmsi.into_inner(),
                latitude,
                longitude,
                course_over_ground,
                rate_of_turn,
                p.true_heading,
                speed_over_ground,
                p.msgtime,
                p.altitude,
                distance_to_shore,
                ais_class,
                p.message_type_id,
                p.navigational_status as i32,
            )
            .execute(&mut *tx)
            .await?;
        }

        let len = inserted.len();
        let mut lat = Vec::with_capacity(len);
        let mut lon = Vec::with_capacity(len);
        let mut timestamp = Vec::with_capacity(len);
        let mut position_type_id = Vec::with_capacity(len);
        let mut mmsi = Vec::with_capacity(len);

        for i in inserted {
            lat.push(i.latitude);
            lon.push(i.longitude);
            timestamp.push(i.timestamp);
            position_type_id.push(PositionType::Ais as i32);
            mmsi.push(i.mmsi);
        }

        // By joining 'fiskeridir_ais_vessel_mapping_whitelist' we risk getting multiple
        // hits as 'call_sign' is not unique on that table. However, it does not matter as one of
        // the rows will be excluded due to our exclusion constraint and the values we are
        // interested in each row are identical. This case will occur when we have added
        // multiple fiskeridir_vessel_ids mapping to the same call sign in our whitelist.
        //
        // The 'where' statement catches 2 cases:
        // - ais_vessels.call_sign is null, we accept this as there are ais vessels with null
        // call signs.
        // - fiskeridir_ais_vessel_mapping_whitelist.fiskeridir_vessel_id is not null, this
        // requires the vessel to exist in our whitelist mapping.
        //
        // So a position either has has to be associated with an ais vessel without call sign or
        // exist in our whitelist to be added to the position area table.
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
    av.call_sign,
    u."timestamp",
    u.position_type_id,
    u.mmsi
FROM
    UNNEST(
        $1::DOUBLE PRECISION[],
        $2::DOUBLE PRECISION[],
        $3::TIMESTAMPTZ[],
        $4::INT[],
        $5::INT[]
    ) u (
        latitude,
        longitude,
        "timestamp",
        position_type_id,
        mmsi
    )
    INNER JOIN ais_vessels av ON av.mmsi = u.mmsi
    LEFT JOIN fiskeridir_ais_vessel_mapping_whitelist f ON av.call_sign = f.call_sign
WHERE
    (
        av.call_sign IS NULL
        OR f.fiskeridir_vessel_id IS NOT NULL
    )
ON CONFLICT DO NOTHING
RETURNING
    a.latitude,
    a.longitude,
    a."timestamp",
    a.mmsi AS "mmsi?: Mmsi",
    a.call_sign
            "#,
            &lat,
            &lon,
            &timestamp,
            &position_type_id,
            &mmsi as &[Mmsi],
        )
        .fetch_all(&mut *tx)
        .await?;

        self.add_ais_vms_aggregated(area_positions_inserted, &mut tx)
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
ON CONFLICT (mmsi) DO
UPDATE
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
