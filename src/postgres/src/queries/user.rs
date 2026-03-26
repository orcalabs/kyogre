use kyogre_core::{BarentswatchUserId, FiskeridirVesselId, User};

use crate::{PostgresAdapter, error::Result};

impl PostgresAdapter {
    pub(crate) async fn get_user_impl(&self, user_id: BarentswatchUserId) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
SELECT
    $1::UUID AS "barentswatch_user_id!: BarentswatchUserId",
    COALESCE(
        (
            SELECT
                ARRAY_AGG(fiskeridir_vessel_id)
            FROM
                user_follows
            WHERE
                barentswatch_user_id = $1
            GROUP BY
                barentswatch_user_id
        ),
        '{}'
    ) AS "following!: Vec<FiskeridirVesselId>",
    (
        SELECT
            fuel_consent
        FROM
            user_settings
        WHERE
            barentswatch_user_id = $1
    ) AS fuel_consent;
            "#,
            user_id.as_ref(),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub(crate) async fn update_user_impl(
        &self,
        user: &kyogre_core::UpdateUser,
        id: BarentswatchUserId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(following) = &user.following {
            self.update_user_follows(id, following, &mut tx).await?;
        }

        if let Some(consent) = user.fuel_consent {
            sqlx::query!(
                r#"
INSERT INTO
    user_settings (barentswatch_user_id, fuel_consent)
VALUES
    ($1, $2)
ON CONFLICT (barentswatch_user_id) DO UPDATE
SET
    fuel_consent = EXCLUDED.fuel_consent
            "#,
                id as BarentswatchUserId,
                consent,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn update_user_follows<'a>(
        &'a self,
        user_id: BarentswatchUserId,
        vessel_ids: &[FiskeridirVesselId],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
DELETE FROM user_follows
WHERE
    barentswatch_user_id = $1
            "#,
            user_id.as_ref(),
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    user_follows (barentswatch_user_id, fiskeridir_vessel_id)
SELECT
    $1,
    *
FROM
    UNNEST($2::BIGINT[])
            "#,
            user_id.as_ref(),
            vessel_ids as &[FiskeridirVesselId],
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
