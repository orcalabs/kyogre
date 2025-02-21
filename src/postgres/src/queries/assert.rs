use crate::{
    PostgresAdapter,
    error::{CallSignDoesNotExistSnafu, Result},
};
use fiskeridir_rs::{CallSign, OrgId};
use kyogre_core::FiskeridirVesselId;

impl PostgresAdapter {
    pub async fn assert_call_sign_is_in_org(
        &self,
        call_sign: &CallSign,
        org_id: OrgId,
    ) -> Result<Option<Vec<FiskeridirVesselId>>> {
        Ok(sqlx::query!(
            r#"
SELECT
    ARRAY_AGG(DISTINCT a2.fiskeridir_vessel_id) AS "ids: Vec<FiskeridirVesselId>"
FROM
    active_vessels a
    INNER JOIN orgs__fiskeridir_vessels o ON o.fiskeridir_vessel_id = a.fiskeridir_vessel_id
    INNER JOIN orgs__fiskeridir_vessels o2 ON o.org_id = o2.org_id
    INNER JOIN active_vessels a2 ON o2.fiskeridir_vessel_id = a2.fiskeridir_vessel_id
WHERE
    a.call_sign = $1
    AND o.org_id = $2
            "#,
            call_sign.as_ref(),
            org_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?
        .ids)
    }
    pub async fn assert_call_sign_exists(
        &self,
        call_sign: &CallSign,
        executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    ) -> Result<()> {
        let exists = sqlx::query!(
            r#"
SELECT
    1 AS EXISTS
FROM
    active_vessels
WHERE
    call_sign = $1
            "#,
            call_sign.as_ref(),
        )
        .fetch_optional(executor)
        .await?
        .is_some();

        if exists {
            Ok(())
        } else {
            CallSignDoesNotExistSnafu {
                call_sign: call_sign.clone(),
            }
            .fail()
        }
    }
}
