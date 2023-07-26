use crate::{models::User, PostgresAdapter};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{BarentswatchUserId, FiskeridirVesselId};

use crate::error::PostgresError;

impl PostgresAdapter {
    pub(crate) async fn get_user_impl(
        &self,
        user_id: BarentswatchUserId,
    ) -> Result<Option<User>, PostgresError> {
        sqlx::query_as!(
            User,
            r#"
SELECT
    barentswatch_user_id,
    ARRAY_AGG(fiskeridir_vessel_id) AS "following!"
FROM
    user_follows
WHERE
    barentswatch_user_id = $1
GROUP BY
    barentswatch_user_id
            "#,
            user_id.0,
        )
        .fetch_optional(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn update_user_impl(
        &self,
        user: kyogre_core::User,
    ) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        self.update_user_follows(user.barentswatch_user_id, user.following, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)
    }

    pub(crate) async fn update_user_follows<'a>(
        &'a self,
        user_id: BarentswatchUserId,
        vessel_ids: Vec<FiskeridirVesselId>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let vessel_ids = vessel_ids.into_iter().map(|id| id.0).collect::<Vec<_>>();

        sqlx::query!(
            r#"
DELETE FROM user_follows
WHERE
    barentswatch_user_id = $1
            "#,
            user_id.0,
        )
        .execute(&mut **tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

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
            user_id.0,
            vessel_ids.as_slice(),
        )
        .execute(&mut **tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
