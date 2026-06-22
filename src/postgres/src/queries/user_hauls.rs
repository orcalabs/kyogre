use crate::{
    PostgresAdapter,
    error::{ObjectNotFoundSnafu, Result},
    models::{StartedUserHaul, UserHaul, UserHaulWithAisPositions},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear};
use futures::TryStreamExt;
use kyogre_core::{
    BarentswatchUserId, FiskeridirVesselId, HaulStart, Object, ProcessingStatus, TripId,
    UpdateUserHaul, UserHaulDistanceUpdate, UserHaulId,
};
use sqlx::{PgTransaction, postgres::types::PgRange};
use std::ops::Bound;

impl PostgresAdapter {
    pub(crate) async fn refresh_user_haul_mappings_impl(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        #[derive(Debug)]
        struct RefreshBoundary {
            vessel_id: FiskeridirVesselId,
            refresh_boundary: DateTime<Utc>,
            current_trip_refresh_boundary: DateTime<Utc>,
        }

        // This has to be wrapped in a transaction as the scraper might modify these entries and we
        // might use an outdated value if this select was not in the same tx as the write.
        let to_refresh = sqlx::query_as!(
            RefreshBoundary,
            r#"
SELECT
    refresh_boundary AS "refresh_boundary!",
    current_trip_refresh_boundary AS "current_trip_refresh_boundary!",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId"
FROM
    user_hauls_refresh_boundary
WHERE
    refresh_boundary IS NOT NULL
    AND current_trip_refresh_boundary IS NOT NULL
            "#
        )
        .fetch_all(&mut *tx)
        .await?;

        if to_refresh.is_empty() {
            return Ok(());
        }

        let mut to_reset = Vec::new();
        for t in to_refresh {
            self.refresh_user_haul_mappings_by_timestamp(t.vessel_id, t.refresh_boundary, &mut tx)
                .await?;
            self.set_trips_refresh_boundary(t.vessel_id, t.refresh_boundary, &mut tx)
                .await?;

            let current_trip_start_before_latest_update = sqlx::query!(
                r#"
SELECT
    departure_timestamp
FROM
    current_trips
WHERE
    fiskeridir_vessel_id = $1
    AND departure_timestamp <= $2
            "#,
                t.vessel_id as FiskeridirVesselId,
                t.current_trip_refresh_boundary
            )
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(dep) = current_trip_start_before_latest_update {
                self.refresh_current_trip_user_hauls(t.vessel_id, dep.departure_timestamp, &mut tx)
                    .await?;
            }
            to_reset.push(t.vessel_id)
        }

        sqlx::query!(
            r#"
UPDATE user_hauls_refresh_boundary
SET
    refresh_boundary = NULL,
    current_trip_refresh_boundary = NULL
WHERE
    fiskeridir_vessel_id = ANY ($1::BIGINT[])
            "#,
            &to_reset as &[FiskeridirVesselId]
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
    pub(crate) async fn set_user_haul_refresh_boundary(
        &self,
        vessel_id: FiskeridirVesselId,
        min_timestamp: DateTime<Utc>,
        max_timestamp: DateTime<Utc>,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    user_hauls_refresh_boundary (
        refresh_boundary,
        current_trip_refresh_boundary,
        fiskeridir_vessel_id
    )
VALUES
    ($1, $2, $3)
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    refresh_boundary = LEAST(
        user_hauls_refresh_boundary.refresh_boundary,
        excluded.refresh_boundary
    ),
    current_trip_refresh_boundary = GREATEST(
        user_hauls_refresh_boundary.current_trip_refresh_boundary,
        excluded.current_trip_refresh_boundary
    )
            "#,
            min_timestamp,
            max_timestamp,
            vessel_id as FiskeridirVesselId,
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
    pub(crate) async fn set_user_haul_refresh_boundary_from_haul_event_ids(
        &self,
        vessel_event_ids: &[i64],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    per_vessel AS (
        SELECT
            MIN(occurence_timestamp) AS min_timestamp,
            MAX(occurence_timestamp) AS max_timestamp,
            fiskeridir_vessel_id
        FROM
            vessel_events
        WHERE
            vessel_event_id = ANY ($1::BIGINT[])
        GROUP BY
            fiskeridir_vessel_id
    )
INSERT INTO
    user_hauls_refresh_boundary (
        refresh_boundary,
        current_trip_refresh_boundary,
        fiskeridir_vessel_id
    )
SELECT
    q.min_timestamp,
    q.max_timestamp,
    q.fiskeridir_vessel_id
FROM
    per_vessel q
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    refresh_boundary = LEAST(
        user_hauls_refresh_boundary.refresh_boundary,
        excluded.refresh_boundary
    ),
    current_trip_refresh_boundary = GREATEST(
        user_hauls_refresh_boundary.current_trip_refresh_boundary,
        excluded.current_trip_refresh_boundary
    )
            "#,
            vessel_event_ids
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
    pub(crate) async fn update_current_user_haul_impl(
        &self,
        call_sign: &CallSign,
        update: &HaulStart,
    ) -> Result<kyogre_core::StartedUserHaul> {
        let HaulStart {
            gear,
            fuel_liter_start,
            config,
        } = update;

        let mut tx = self.pool.begin().await?;

        self.assert_user_haul_is_in_progress(call_sign, &mut tx)
            .await?;

        let haul = sqlx::query_as!(
            StartedUserHaul,
            r#"
UPDATE user_hauls
SET
    config = $1,
    start_fuel_liter = $2,
    gear_id = $3
WHERE
    call_sign = $4
    AND end_ts IS NULL
    AND end_fuel_liter IS NULL
RETURNING
    user_haul_id AS "id: UserHaulId",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    gear_id AS "gear: Gear",
    start_ts,
    start_fuel_liter,
    config
            "#,
            config,
            *fuel_liter_start as i32,
            *gear as Gear,
            &call_sign,
        )
        .fetch_one(&mut *tx)
        .await?;

        self.invalidate_fuel(haul.vessel_id, haul.start_ts, None, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(haul.into())
    }

    pub(crate) async fn update_user_haul_impl(
        &self,
        call_sign: &CallSign,
        id: UserHaulId,
        update: &UpdateUserHaul,
    ) -> Result<kyogre_core::UserHaul> {
        let UpdateUserHaul {
            gear,
            config,
            start_ts,
            end_ts,
            start_fuel_liter,
            end_fuel_liter,
            total_living_weight_kg,
        } = update;

        let mut tx = self.pool.begin().await?;

        let prev = sqlx::query!(
            r#"
SELECT
    start_ts,
    end_ts
FROM
    user_hauls
WHERE
    user_haul_id = $1
    AND call_sign = $2
            "#,
            id as UserHaulId,
            &call_sign,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ObjectNotFoundSnafu {
                object: Object::UserHaul(id, call_sign.clone()),
            }
            .build()
        })?;

        self.assert_is_not_current_active_haul(call_sign, id, &mut tx)
            .await?;

        let haul = sqlx::query_as!(
            UserHaul,
            r#"
UPDATE user_hauls
SET
    config = $1,
    start_ts = $2,
    end_ts = $3,
    start_fuel_liter = $4,
    end_fuel_liter = $5,
    total_living_weight_kg = $6,
    gear_id = $7
WHERE
    user_haul_id = $8
    AND call_sign = $9
RETURNING
    user_haul_id AS "id: UserHaulId",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    gear_id AS "gear: Gear",
    start_ts,
    end_ts AS "end_ts!",
    start_fuel_liter,
    end_fuel_liter AS "end_fuel_liter!",
    config,
    total_living_weight_kg
            "#,
            config,
            start_ts,
            end_ts,
            *start_fuel_liter as i32,
            *end_fuel_liter as i32,
            *total_living_weight_kg,
            *gear as Gear,
            id as UserHaulId,
            &call_sign,
        )
        .fetch_one(&mut *tx)
        .await?;

        let start = haul.start_ts.min(prev.start_ts);
        let end = haul.end_ts.max(prev.end_ts.unwrap_or(haul.end_ts));

        self.invalidate_fuel(haul.vessel_id, start, Some(end), &mut tx)
            .await?;

        self.set_user_haul_refresh_boundary(
            haul.vessel_id,
            start,
            haul.start_ts.max(prev.start_ts),
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        Ok(haul.into())
    }

    pub(crate) async fn update_user_haul_distances_impl(
        &self,
        update: Vec<UserHaulDistanceUpdate>,
    ) -> Result<()> {
        let mut id = Vec::with_capacity(update.len());
        let mut distance = Vec::with_capacity(update.len());

        let mut set_to_attempted = Vec::new();

        for u in update {
            if let Some(dist) = u.distance_meters {
                id.push(u.id);
                distance.push(dist as i32);
            } else {
                set_to_attempted.push(u.id);
            }
        }
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
UPDATE user_hauls
SET
    distance_processing_status = $1
WHERE
    user_haul_id = ANY ($2)
        "#,
            ProcessingStatus::Attempted as i32,
            &set_to_attempted as &[UserHaulId]
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
WITH
    hauls_to_update AS (
        SELECT
            UNNEST($1::INT[]) id,
            UNNEST($2::INT[]) distance
    ),
    updated_user_hauls AS (
        UPDATE user_hauls u
        SET
            distance = q.distance,
            distance_processing_status = $3
        FROM
            hauls_to_update q
        WHERE
            u.user_haul_id = q.id
        RETURNING
            u.start_ts,
            u.end_ts,
            u.fiskeridir_vessel_id
    )
INSERT INTO
    user_hauls_refresh_boundary (
        refresh_boundary,
        current_trip_refresh_boundary,
        fiskeridir_vessel_id
    )
SELECT
    MIN(start_ts),
    MAX(start_ts),
    fiskeridir_vessel_id
FROM
    updated_user_hauls
GROUP BY
    fiskeridir_vessel_id
ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
SET
    refresh_boundary = LEAST(
        user_hauls_refresh_boundary.refresh_boundary,
        excluded.refresh_boundary
    ),
    current_trip_refresh_boundary = GREATEST(
        user_hauls_refresh_boundary.current_trip_refresh_boundary,
        excluded.current_trip_refresh_boundary
    )
        "#,
            &id as &[UserHaulId],
            &distance,
            ProcessingStatus::Successful as i32
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn user_hauls_without_distance_impl(
        &self,
    ) -> Result<Vec<UserHaulWithAisPositions>> {
        let hauls = sqlx::query_as!(
            UserHaulWithAisPositions,
            r#"
SELECT
    user_haul_id AS "id!: UserHaulId",
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT('longitude', p.longitude, 'latitude', p.latitude)
            ORDER BY
                p.timestamp
        ),
        '[]'
    )::TEXT AS "ais_positions!"
FROM
    user_hauls u
    INNER JOIN all_vessels v ON u.fiskeridir_vessel_id = v.fiskeridir_vessel_id
    INNER JOIN ais_vessels a ON a.mmsi = v.mmsi
    INNER JOIN ais_positions p ON p.mmsi = a.mmsi
    AND p.timestamp BETWEEN u.start_ts AND u.end_ts
WHERE
    u.distance_processing_status = $1
    --! We want to ensure that all ais positions have been added before attempting to compute
    --! distance, which we ballpark to 10 minutes into the future
    AND NOW() - u.end_ts >= Interval '10 minutes'
GROUP BY
    u.user_haul_id
            "#,
            ProcessingStatus::Unprocessed as i32
        )
        .fetch_all(self.no_plan_cache_pool())
        .await?;

        Ok(hauls)
    }

    pub(crate) async fn set_user_hauls_start_positions_impl(&self) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    hauls_to_update AS (
        SELECT DISTINCT
            ON (u.user_haul_id) u.user_haul_id,
            u.start_ts,
            u.end_ts,
            u.fiskeridir_vessel_id,
            p.latitude,
            p.longitude
        FROM
            user_hauls u
            INNER JOIN all_vessels v ON u.fiskeridir_vessel_id = v.fiskeridir_vessel_id
            INNER JOIN ais_vessels a ON a.mmsi = v.mmsi
            INNER JOIN ais_positions p ON p.mmsi = a.mmsi
            AND p.timestamp BETWEEN u.start_ts - INTERVAL '5 minutes' AND u.start_ts  + INTERVAL '5 minutes'
        WHERE
            u.start_latitude IS NULL
        ORDER BY
            u.user_haul_id,
            ABS(
                EXTRACT(
                    EPOCH
                    FROM
                        p.timestamp
                ) - EXTRACT(
                    EPOCH
                    FROM
                        u.start_ts
                )
            )
    ),
    update_boundaries AS (
        INSERT INTO
            user_hauls_refresh_boundary (
                refresh_boundary,
                current_trip_refresh_boundary,
                fiskeridir_vessel_id
            )
        SELECT
            MIN(start_ts),
            MAX(start_ts),
            fiskeridir_vessel_id
        FROM
            hauls_to_update
        GROUP BY
            fiskeridir_vessel_id
        ON CONFLICT (fiskeridir_vessel_id) DO UPDATE
        SET
            refresh_boundary = LEAST(
                user_hauls_refresh_boundary.refresh_boundary,
                excluded.refresh_boundary
            ),
            current_trip_refresh_boundary = GREATEST(
                user_hauls_refresh_boundary.current_trip_refresh_boundary,
                excluded.current_trip_refresh_boundary
            )
    )
UPDATE user_hauls u
SET
    start_longitude = q.longitude,
    start_latitude = q.latitude
FROM
    hauls_to_update q
WHERE
    q.user_haul_id = u.user_haul_id
        "#,
        )
        .execute(self.no_plan_cache_pool())
        .await?;

        Ok(())
    }
    pub(crate) async fn delete_user_haul_impl(
        &self,
        call_sign: &CallSign,
        id: UserHaulId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        self.assert_is_not_current_active_haul(call_sign, id, &mut tx)
            .await?;

        let deleted = sqlx::query!(
            r#"
DELETE FROM user_hauls
WHERE
    call_sign = $1
    AND user_haul_id = $2
RETURNING
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    start_ts,
    end_ts
            "#,
            call_sign,
            id as UserHaulId,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ObjectNotFoundSnafu {
                object: Object::UserHaul(id, call_sign.clone()),
            }
            .build()
        })?;

        self.invalidate_fuel(deleted.vessel_id, deleted.start_ts, deleted.end_ts, &mut tx)
            .await?;

        self.set_user_haul_refresh_boundary(
            deleted.vessel_id,
            deleted.start_ts,
            deleted.start_ts,
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn user_hauls_impl(
        &self,
        call_sign: &CallSign,
    ) -> Result<Vec<kyogre_core::UserHaul>> {
        let user_hauls = sqlx::query_as!(
            UserHaul,
            r#"
SELECT
    user_haul_id AS "id: UserHaulId",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    gear_id AS "gear: Gear",
    start_ts,
    end_ts AS "end_ts!",
    start_fuel_liter,
    end_fuel_liter AS "end_fuel_liter!",
    config,
    total_living_weight_kg
FROM
    user_hauls
WHERE
    call_sign = $1
    AND end_ts IS NOT NULL
    AND end_fuel_liter IS NOT NULL
ORDER BY
    start_ts
            "#,
            call_sign
        )
        .fetch(&self.pool)
        .map_ok(|h| h.into())
        .try_collect::<Vec<_>>()
        .await?;

        Ok(user_hauls)
    }

    pub(crate) async fn current_user_haul_impl(
        &self,
        call_sign: &CallSign,
    ) -> Result<Option<kyogre_core::StartedUserHaul>> {
        let haul = sqlx::query_as!(
            StartedUserHaul,
            r#"
SELECT
    user_haul_id AS "id: UserHaulId",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    gear_id AS "gear: Gear",
    start_ts,
    start_fuel_liter,
    config
FROM
    user_hauls
WHERE
    end_ts IS NULL
    AND end_fuel_liter IS NULL
    AND call_sign = $1
            "#,
            &call_sign,
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|h| h.into());

        Ok(haul)
    }

    pub(crate) async fn start_user_haul_impl(
        &self,
        call_sign: &CallSign,
        barentswatch_user_id: BarentswatchUserId,
        start: &kyogre_core::HaulStart,
    ) -> Result<kyogre_core::StartedUserHaul> {
        let kyogre_core::HaulStart {
            gear,
            fuel_liter_start,
            config,
        } = start;

        let mut tx = self.pool.begin().await?;

        self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let haul = sqlx::query_as!(
            StartedUserHaul,
            r#"
INSERT INTO
    user_hauls (
        fiskeridir_vessel_id,
        call_sign,
        gear_id,
        start_ts,
        start_fuel_liter,
        config,
        barentswatch_user_id
    )
SELECT
    a.fiskeridir_vessel_id,
    $1,
    $2,
    $3,
    $4,
    $5,
    $6
FROM
    active_vessels a
WHERE
    a.call_sign = $1
RETURNING
    user_haul_id AS "id: UserHaulId",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    gear_id AS "gear: Gear",
    start_ts,
    start_fuel_liter,
    config
            "#,
            &call_sign,
            *gear as Gear,
            Utc::now(),
            *fuel_liter_start as i32,
            config,
            barentswatch_user_id as BarentswatchUserId,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(haul.into())
    }

    pub(crate) async fn abort_user_haul_impl(&self, call_sign: &CallSign) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        self.assert_user_haul_is_in_progress(call_sign, &mut tx)
            .await?;

        let deleted = sqlx::query!(
            r#"
DELETE FROM user_hauls
WHERE
    call_sign = $1
    AND end_ts IS NULL
    AND end_fuel_liter IS NULL
RETURNING
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    start_ts,
    end_ts
            "#,
            &call_sign,
        )
        .fetch_one(&mut *tx)
        .await?;

        self.invalidate_fuel(deleted.vessel_id, deleted.start_ts, deleted.end_ts, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn stop_user_haul_impl(
        &self,
        call_sign: &CallSign,
        end: &kyogre_core::HaulEnd,
        barentswatch_user_id: BarentswatchUserId,
    ) -> Result<kyogre_core::UserHaul> {
        let kyogre_core::HaulEnd {
            fuel_liter_end,
            total_living_weight_kg,
        } = end;

        let mut tx = self.pool.begin().await?;

        self.assert_user_haul_is_in_progress(call_sign, &mut tx)
            .await?;

        let haul = sqlx::query_as!(
            UserHaul,
            r#"
UPDATE user_hauls
SET
    end_ts = $1,
    end_fuel_liter = $2,
    total_living_weight_kg = $3
WHERE
    call_sign = $4
    AND end_ts IS NULL
    AND end_fuel_liter IS NULL
RETURNING
    user_haul_id AS "id: UserHaulId",
    fiskeridir_vessel_id AS "vessel_id: FiskeridirVesselId",
    gear_id AS "gear: Gear",
    start_ts,
    end_ts AS "end_ts!",
    start_fuel_liter,
    end_fuel_liter AS "end_fuel_liter!",
    config,
    total_living_weight_kg
            "#,
            Utc::now(),
            *fuel_liter_end as i32,
            *total_living_weight_kg,
            &call_sign
        )
        .fetch_one(&mut *tx)
        .await?;

        self.set_user_haul_refresh_boundary(haul.vessel_id, haul.start_ts, haul.start_ts, &mut tx)
            .await?;

        let haul: kyogre_core::UserHaul = haul.into();

        let start_fuel = kyogre_core::CreateFuelMeasurement {
            timestamp: haul.start_ts,
            fuel_liter: haul.start_fuel_liter as f64,
            fuel_after_liter: None,
        };
        let end_fuel = kyogre_core::CreateFuelMeasurement {
            timestamp: haul.end_ts,
            fuel_liter: haul.end_fuel_liter as f64,
            fuel_after_liter: None,
        };

        self.add_fuel_measurements_tx(
            &[start_fuel, end_fuel],
            call_sign,
            barentswatch_user_id,
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        Ok(haul)
    }

    async fn invalidate_fuel(
        &self,
        vessel_id: FiskeridirVesselId,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
        tx: &mut PgTransaction<'_>,
    ) -> Result<()> {
        let range = PgRange {
            start: Bound::Included(start),
            end: end.map(Bound::Included).unwrap_or(Bound::Unbounded),
        };

        sqlx::query!(
            r#"
UPDATE fuel_estimates
SET
    status = $1
WHERE
    fiskeridir_vessel_id = $2
    AND day_range && $3
            "#,
            ProcessingStatus::Unprocessed as i32,
            vessel_id as FiskeridirVesselId,
            range,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM live_fuel
WHERE
    fiskeridir_vessel_id = $1
    AND latest_position_timestamp >= $2
            "#,
            vessel_id as FiskeridirVesselId,
            start,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
UPDATE trips
SET
    trip_position_fuel_consumption_distribution_status = $1
WHERE
    fiskeridir_vessel_id = $2
    AND period && $3
            "#,
            ProcessingStatus::Unprocessed as i32,
            vessel_id as FiskeridirVesselId,
            range,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_user_haul_trip_mappings(
        &self,
        trip_ids: &[TripId],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
WITH
    overlapping AS (
        SELECT
            ANY_VALUE (t.trip_id) AS trip_id,
            u.user_haul_id
        FROM
            trips t
            INNER JOIN user_hauls u ON t.period && u.period
            AND u.fiskeridir_vessel_id = t.fiskeridir_vessel_id
        WHERE
            t.trip_id = ANY ($1::BIGINT[])
            AND u.haul_id IS NULL
        GROUP BY
            u.user_haul_id
        HAVING
            COUNT(DISTINCT t.trip_id) = 1
    )
UPDATE user_hauls u
SET
    trip_id = o.trip_id
FROM
    overlapping o
WHERE
    o.user_haul_id = u.user_haul_id
        "#,
            trip_ids as &[TripId]
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn refresh_user_haul_mappings_by_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: DateTime<Utc>,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let user_haul_ids: Vec<_> = sqlx::query!(
            r#"
UPDATE user_hauls
SET
    haul_id = NULL,
    trip_id = NULL
WHERE
    fiskeridir_vessel_id = $1
    AND end_ts >= $2
RETURNING
    user_haul_id
        "#,
            vessel_id as FiskeridirVesselId,
            timestamp
        )
        .fetch(&mut **tx)
        .map_ok(|r| r.user_haul_id)
        .try_collect()
        .await?;

        if user_haul_ids.is_empty() {
            return Ok(());
        }

        sqlx::query!(
            r#"
WITH
    overlapping AS (
        SELECT
            ANY_VALUE (q.haul_id) AS haul_id,
            q.user_haul_id
        FROM
            (
                SELECT
                    h.haul_id,
                    ANY_VALUE (u.user_haul_id) AS user_haul_id,
                    EXTRACT(
                        EPOCH
                        FROM
                            (
                                UPPER(h.period * ANY_VALUE (u.period)) - LOWER(h.period * ANY_VALUE (u.period))
                            )
                    ) AS overlap
                FROM
                    user_hauls u
                    INNER JOIN hauls h ON h.period && u.period
                    AND u.fiskeridir_vessel_id = h.fiskeridir_vessel_id
                WHERE
                    u.user_haul_id = ANY ($1::INT[])
                GROUP BY
                    h.haul_id
                HAVING
                    COUNT(DISTINCT u.user_haul_id) = 1
            ) q
        GROUP BY
            q.user_haul_id
        HAVING
            COUNT(DISTINCT q.haul_id) = 1
    )
UPDATE user_hauls u
SET
    haul_id = o.haul_id,
    trip_id = NULL
FROM
    overlapping o
WHERE
    o.user_haul_id = u.user_haul_id
        "#,
            &user_haul_ids
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
WITH
    overlapping AS (
        SELECT
            ANY_VALUE (t.trip_id) AS trip_id,
            u.user_haul_id
        FROM
            user_hauls u
            INNER JOIN trips t ON t.period && u.period
            AND u.fiskeridir_vessel_id = t.fiskeridir_vessel_id
        WHERE
            u.user_haul_id = ANY ($1)
            AND u.haul_id IS NULL
        GROUP BY
            u.user_haul_id
        HAVING
            COUNT(DISTINCT t.trip_id) = 1
    )
UPDATE user_hauls u
SET
    trip_id = o.trip_id
FROM
    overlapping o
WHERE
    o.user_haul_id = u.user_haul_id
        "#,
            &user_haul_ids
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
