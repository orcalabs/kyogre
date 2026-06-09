use std::ops::Bound;

use crate::{
    PostgresAdapter,
    error::{ObjectNotFoundSnafu, Result},
    models::{StartedUserHaul, UserHaul},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, Gear};
use futures::TryStreamExt;
use kyogre_core::{
    BarentswatchUserId, FiskeridirVesselId, HaulStart, Object, ProcessingStatus, UpdateUserHaul,
    UserHaulId,
};
use sqlx::{PgTransaction, postgres::types::PgRange};

impl PostgresAdapter {
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

        tx.commit().await?;

        Ok(haul.into())
    }

    pub(crate) async fn delete_user_haul_impl(
        &self,
        call_sign: &CallSign,
        id: UserHaulId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

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
}
