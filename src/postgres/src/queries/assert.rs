use crate::{error::Result, PostgresAdapter};
use fiskeridir_rs::{CallSign, OrgId};

impl PostgresAdapter {
    pub async fn assert_call_sign_is_in_org(
        &self,
        call_sign: &CallSign,
        org_id: OrgId,
    ) -> Result<bool> {
        Ok(sqlx::query!(
            r#"
SELECT
    1 as exists
FROM
    fiskeridir_ais_vessel_mapping_whitelist w
    INNER JOIN orgs__fiskeridir_vessels o ON o.fiskeridir_vessel_id = w.fiskeridir_vessel_id
WHERE
    w.call_sign = $1
    AND o.org_id = $2
            "#,
            call_sign.as_ref(),
            org_id.into_inner()
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some())
    }
}
