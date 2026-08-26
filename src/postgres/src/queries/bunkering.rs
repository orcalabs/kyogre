use crate::{
    PostgresAdapter,
    error::{BunkeringAlreadyExistsSnafu, ObjectNotFoundSnafu, Result},
};
use fiskeridir_rs::CallSign;
use kyogre_core::{BarentswatchUserId, BunkeringId, CreateBunkering, FiskeridirVesselId, Object};

impl PostgresAdapter {
    pub(crate) async fn add_bunkering_impl(
        &self,
        bunkering: &CreateBunkering,
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Result<kyogre_core::Bunkering> {
        let mut tx = self.pool.begin().await?;

        let id = self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let exists = sqlx::query!(
            r#"
SELECT
    TRUE AS bunkering_exists
FROM
    bunkerings
WHERE
    fiskeridir_vessel_id = $1
    AND timestamp = $2
            "#,
            id as FiskeridirVesselId,
            bunkering.timestamp,
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        if exists {
            return BunkeringAlreadyExistsSnafu {
                ts: bunkering.timestamp,
            }
            .fail();
        }

        let out = sqlx::query_as!(
            kyogre_core::Bunkering,
            r#"
INSERT INTO
    bunkerings (
        fuel_liter,
        timestamp,
        fiskeridir_vessel_id,
        barentswatch_user_id
    )
VALUES
    ($1, $2, $3, $4)
RETURNING
    bunkering_id AS "id: BunkeringId",
    fuel_liter,
    timestamp
            "#,
            bunkering.fuel_liter,
            bunkering.timestamp,
            id as FiskeridirVesselId,
            user_id as BarentswatchUserId
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(out)
    }

    pub(crate) async fn update_bunkering_impl(
        &self,
        id: BunkeringId,
        bunkering: &CreateBunkering,
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let vessel_id = self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let exists = sqlx::query!(
            r#"
SELECT
    TRUE AS bunkering_exists
FROM
    bunkerings
WHERE
    fiskeridir_vessel_id = $1
    AND timestamp = $2
    AND bunkering_id != $3
            "#,
            vessel_id as FiskeridirVesselId,
            bunkering.timestamp,
            id as BunkeringId
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        if exists {
            return BunkeringAlreadyExistsSnafu {
                ts: bunkering.timestamp,
            }
            .fail();
        }

        let rows_affected = sqlx::query!(
            r#"
UPDATE bunkerings
SET
    fuel_liter = $1,
    timestamp = $2,
    barentswatch_user_id = $3
WHERE
    bunkering_id = $4
    AND fiskeridir_vessel_id = $5
            "#,
            bunkering.fuel_liter,
            bunkering.timestamp,
            user_id as BarentswatchUserId,
            id as BunkeringId,
            vessel_id as FiskeridirVesselId
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return ObjectNotFoundSnafu {
                object: Object::Bunkering(id),
            }
            .fail();
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn delete_bunkering_impl(
        &self,
        id: BunkeringId,
        call_sign: &CallSign,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let vessel_id = self.assert_call_sign_exists(call_sign, &mut *tx).await?;

        let rows_affected = sqlx::query!(
            r#"
DELETE FROM bunkerings
WHERE
    bunkering_id = $1
    AND fiskeridir_vessel_id = $2
            "#,
            id as BunkeringId,
            vessel_id as FiskeridirVesselId
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return ObjectNotFoundSnafu {
                object: Object::Bunkering(id),
            }
            .fail();
        }

        tx.commit().await?;

        Ok(())
    }
}
