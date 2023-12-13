use std::collections::HashMap;

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Utc};
use futures::{Stream, TryStreamExt};
use kyogre_core::{
    AisPermission, AisVesselMigrate, DateRange, Mmsi, NavigationStatus, NewAisPosition,
    NewAisStatic, LEISURE_VESSEL_LENGTH_AIS_BOUNDARY, LEISURE_VESSEL_SHIP_TYPES,
    PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY,
};
use unnest_insert::UnnestInsert;

use crate::{
    error::{BigDecimalError, PostgresError},
    models::{AisClass, AisPosition, NewAisVessel, NewAisVesselHistoric},
    PostgresAdapter,
};
use error_stack::{report, Result, ResultExt};

impl PostgresAdapter {
    pub(crate) async fn all_ais_impl(&self) -> Result<Vec<AisPosition>, PostgresError> {
        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    mmsi,
    TIMESTAMP AS msgtime,
    course_over_ground,
    navigation_status_id AS "navigational_status: NavigationStatus",
    rate_of_turn,
    speed_over_ground,
    true_heading,
    distance_to_shore
FROM
    ais_positions
ORDER BY
    TIMESTAMP ASC
            "#,
        )
        .fetch_all(self.ais_pool())
        .await
        .change_context(PostgresError::Query)
    }
    pub(crate) fn ais_positions_impl(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        permission: AisPermission,
    ) -> impl Stream<Item = Result<AisPosition, PostgresError>> + '_ {
        sqlx::query_as!(
            AisPosition,
            r#"
SELECT
    latitude,
    longitude,
    mmsi,
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
            mmsi.0,
            range.start(),
            range.end(),
            LEISURE_VESSEL_SHIP_TYPES.as_slice(),
            LEISURE_VESSEL_LENGTH_AIS_BOUNDARY as i32,
            permission as i32,
            PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY as i32,
        )
        .fetch(self.ais_pool())
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn add_ais_positions(
        &self,
        positions: &[NewAisPosition],
    ) -> Result<(), PostgresError> {
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
            if let Some(v) = latest_position_per_vessel.get(&p.mmsi) {
                if p.msgtime > v.msgtime {
                    latest_position_per_vessel.insert(p.mmsi, p.clone());
                }
            } else {
                latest_position_per_vessel.insert(p.mmsi, p.clone());
            }

            mmsis.push(p.mmsi.0);
            latitude.push(
                BigDecimal::from_f64(p.latitude)
                    .ok_or(BigDecimalError(p.latitude))
                    .change_context(PostgresError::DataConversion)?,
            );
            longitude.push(
                BigDecimal::from_f64(p.longitude)
                    .ok_or(BigDecimalError(p.longitude))
                    .change_context(PostgresError::DataConversion)?,
            );
            course_over_ground.push(
                p.course_over_ground
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );
            rate_of_turn.push(
                p.rate_of_turn
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );

            true_heading.push(p.true_heading);
            speed_over_ground.push(
                p.speed_over_ground
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );
            altitude.push(p.altitude);
            distance_to_shore.push(
                BigDecimal::from_f64(p.distance_to_shore)
                    .ok_or(BigDecimalError(p.distance_to_shore))
                    .change_context(PostgresError::DataConversion)?,
            );
            navigation_status_id.push(p.navigational_status as i32);
            timestamp.push(p.msgtime);
            ais_class.push(p.ais_class.map(|a| AisClass::from(a).to_string()));
            ais_message_type.push(p.message_type_id);
        }

        let mut tx = self.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    ais_vessels (mmsi)
VALUES
    (UNNEST($1::INT[]))
ON CONFLICT (mmsi) DO NOTHING
            "#,
            &mmsis
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

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
        $2::DECIMAL[],
        $3::DECIMAL[],
        $4::DECIMAL[],
        $5::DECIMAL[],
        $6::INT[],
        $7::DECIMAL[],
        $8::TIMESTAMPTZ[],
        $9::INT[],
        $10::DECIMAL[],
        $11::VARCHAR[],
        $12::INT[],
        $13::INT[]
    )
ON CONFLICT (mmsi, TIMESTAMP) DO NOTHING
            "#,
            &mmsis,
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
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

        for (_, p) in latest_position_per_vessel {
            let latitude = BigDecimal::from_f64(p.latitude)
                .ok_or(BigDecimalError(p.latitude))
                .change_context(PostgresError::DataConversion)?;

            let longitude = BigDecimal::from_f64(p.longitude)
                .ok_or(BigDecimalError(p.longitude))
                .change_context(PostgresError::DataConversion)?;

            let course_over_ground = p
                .course_over_ground
                .map(|v| {
                    BigDecimal::from_f64(v)
                        .ok_or(BigDecimalError(v))
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?;

            let rate_of_turn = p
                .rate_of_turn
                .map(|v| {
                    BigDecimal::from_f64(v)
                        .ok_or(BigDecimalError(v))
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?;

            let speed_over_ground = p
                .speed_over_ground
                .map(|v| {
                    BigDecimal::from_f64(v)
                        .ok_or(BigDecimalError(v))
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?;

            let distance_to_shore = BigDecimal::from_f64(p.distance_to_shore)
                .ok_or(BigDecimalError(p.distance_to_shore))
                .change_context(PostgresError::DataConversion)?;

            let ais_class = p.ais_class.map(|a| AisClass::from(a).to_string());

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
        $2::DECIMAL,
        $3::DECIMAL,
        $4::DECIMAL,
        $5::DECIMAL,
        $6::INT,
        $7::DECIMAL,
        $8::timestamptz,
        $9::INT,
        $10::DECIMAL,
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
                p.mmsi.0,
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
            .await
            .change_context(PostgresError::Query)?;
        }

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn ais_vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, PostgresError> {
        Ok(sqlx::query_as!(
            crate::models::AisVesselMigrationProgress,
            r#"
SELECT
    mmsi,
    progress
FROM
    ais_data_migration_progress
WHERE
    progress < $1
            "#,
            migration_end_threshold
        )
        .fetch_all(&self.pool)
        .await
        .change_context(PostgresError::Query)?
        .into_iter()
        .map(|v| AisVesselMigrate {
            mmsi: Mmsi(v.mmsi),
            progress: v.progress,
        })
        .collect())
    }

    pub(crate) async fn add_ais_vessels(
        &self,
        static_messages: &[NewAisStatic],
    ) -> Result<(), PostgresError> {
        let mut unique_static: HashMap<Mmsi, NewAisStatic> = HashMap::new();
        for v in static_messages {
            unique_static.entry(v.mmsi).or_insert(v.clone());
        }

        let mut tx = self.begin().await?;

        let vessels = unique_static
            .into_values()
            .map(NewAisVessel::from)
            .collect();

        let vessels_historic = static_messages
            .iter()
            .cloned()
            .map(NewAisVesselHistoric::from)
            .collect();

        NewAisVessel::unnest_insert(vessels, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        NewAisVesselHistoric::unnest_insert(vessels_historic, &mut *tx)
            .await
            .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
    pub(crate) async fn add_mmsis_impl(&self, mmsis: Vec<Mmsi>) -> Result<(), PostgresError> {
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
            &mmsis.into_iter().map(|v| v.0).collect::<Vec<i32>>()
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_ais_migration_data(
        &self,
        mmsi: Mmsi,
        positions: Vec<kyogre_core::AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), PostgresError> {
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
            mmsis.push(p.mmsi.0);
            latitude.push(
                BigDecimal::from_f64(p.latitude)
                    .ok_or(BigDecimalError(p.latitude))
                    .change_context(PostgresError::DataConversion)?,
            );
            longitude.push(
                BigDecimal::from_f64(p.longitude)
                    .ok_or(BigDecimalError(p.longitude))
                    .change_context(PostgresError::DataConversion)?,
            );
            course_over_ground.push(
                p.course_over_ground
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );
            rate_of_turn.push(
                p.rate_of_turn
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );

            true_heading.push(p.true_heading);
            speed_over_ground.push(
                p.speed_over_ground
                    .map(|v| {
                        BigDecimal::from_f64(v)
                            .ok_or(BigDecimalError(v))
                            .change_context(PostgresError::DataConversion)
                    })
                    .transpose()?,
            );
            distance_to_shore.push(
                BigDecimal::from_f64(p.distance_to_shore)
                    .ok_or(BigDecimalError(p.distance_to_shore))
                    .change_context(PostgresError::DataConversion)?,
            );
            navigation_status_id.push(p.navigational_status.map(|v| v as i32));
            timestamp.push(p.msgtime);
        }

        let mut tx = self.begin().await?;

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
            &mmsi.0,
            &progress
        )
        .execute(&mut *tx)
        .await
        .change_context(PostgresError::Query)?;

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
        $2::DECIMAL[],
        $3::DECIMAL[],
        $4::DECIMAL[],
        $5::DECIMAL[],
        $6::INT[],
        $7::DECIMAL[],
        $8::TIMESTAMPTZ[],
        $9::DECIMAL[],
        $10::INT[]
    )
ON CONFLICT (mmsi, TIMESTAMP) DO NOTHING
            "#,
            &mmsis,
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
        .await
        .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }
}
