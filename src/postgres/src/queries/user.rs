use kyogre_core::{BarentswatchUserId, FiskeridirVesselId, User};

use crate::{PostgresAdapter, error::Result};

impl PostgresAdapter {
    pub(crate) async fn get_user_impl(&self, user_id: BarentswatchUserId) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
SELECT
    barentswatch_user_id AS "barentswatch_user_id!: BarentswatchUserId",
    ARRAY_AGG(fiskeridir_vessel_id) AS "following!: Vec<FiskeridirVesselId>"
FROM
    user_follows
WHERE
    barentswatch_user_id = $1
GROUP BY
    barentswatch_user_id
            "#,
            user_id.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub(crate) async fn update_user_impl(&self, user: &kyogre_core::User) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        self.update_user_follows(user.barentswatch_user_id, &user.following, &mut tx)
            .await?;

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
