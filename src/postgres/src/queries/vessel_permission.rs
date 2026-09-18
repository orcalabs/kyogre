use crate::{PostgresAdapter, error::Result, models::VesselPermission};

impl PostgresAdapter {
    pub(crate) async fn add_vessel_permissions_impl(
        &self,
        permissions: Vec<fiskeridir_rs::VesselPermission>,
    ) -> Result<()> {
        self.unnest_insert_from::<_, _, VesselPermission>(permissions, &self.pool)
            .await?;
        Ok(())
    }
}
