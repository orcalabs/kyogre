use crate::{
    PostgresAdapter,
    error::{ObjectNotFoundSnafu, Result},
    models::{StartedUserHaul, UserHaul},
};
use chrono::Utc;
use fiskeridir_rs::CallSign;
use futures::TryStreamExt;
use kyogre_core::{BarentswatchUserId, HaulStart, Object, UpdateUserHaul, UserHaulId};

impl PostgresAdapter {
    pub(crate) async fn update_current_user_haul_impl(
        &self,
        call_sign: &CallSign,
        update: &HaulStart,
    ) -> Result<kyogre_core::StartedUserHaul> {
        let HaulStart {
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
    start_fuel_liter = $2
WHERE
    call_sign = $3
    AND end_ts IS NULL
    AND end_fuel_liter IS NULL
RETURNING
    user_haul_id AS "id: UserHaulId",
    start_ts,
    start_fuel_liter,
    config
            "#,
            config,
            *fuel_liter_start as i32,
            &call_sign
        )
        .fetch_one(&mut *tx)
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
            config,
            start_ts,
            end_ts,
            start_fuel_liter,
            end_fuel_liter,
            total_living_weight_kg,
        } = update;

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
    total_living_weight_kg = $6
WHERE
    user_haul_id = $7
    AND call_sign = $8
RETURNING
    user_haul_id AS "id: UserHaulId",
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
            id as UserHaulId,
            &call_sign
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(haul) = haul {
            Ok(haul.into())
        } else {
            ObjectNotFoundSnafu {
                object: Object::UserHaul(id),
            }
            .fail()
        }
    }
    pub(crate) async fn delete_user_haul_impl(
        &self,
        call_sign: &CallSign,
        id: UserHaulId,
    ) -> Result<()> {
        let affected = sqlx::query!(
            r#"
DELETE FROM user_hauls
WHERE
    call_sign = $1
    AND user_haul_id = $2
            "#,
            call_sign,
            id as UserHaulId
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if affected == 0 {
            ObjectNotFoundSnafu {
                object: Object::UserHaul(id),
            }
            .fail()
        } else {
            Ok(())
        }
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
            fuel_liter_start,
            config,
        } = start;

        let mut tx = self.pool.begin().await?;

        let haul = sqlx::query_as!(
            StartedUserHaul,
            r#"
INSERT INTO
    user_hauls (
        fiskeridir_vessel_id,
        call_sign,
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
    $5
FROM
    active_vessels a
WHERE
    a.call_sign = $1
RETURNING
    user_haul_id AS "id: UserHaulId",
    start_ts,
    start_fuel_liter,
    config
            "#,
            &call_sign,
            Utc::now(),
            *fuel_liter_start as i32,
            config,
            barentswatch_user_id as BarentswatchUserId
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

        sqlx::query!(
            r#"
DELETE FROM user_hauls
WHERE
    call_sign = $1
    AND end_ts IS NULL
    AND end_fuel_liter IS NULL
            "#,
            &call_sign,
        )
        .execute(&mut *tx)
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
}
