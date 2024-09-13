use std::collections::HashSet;

use futures::TryStreamExt;
use kyogre_core::VesselEventType;

use crate::{error::Result, ers_tra_set::ErsTraSet, models::NewErsTra, PostgresAdapter};

impl PostgresAdapter {
    pub(crate) async fn add_ers_tra_set(&self, set: ErsTraSet<'_>) -> Result<()> {
        let prepared_set = set.prepare();

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(prepared_set.ers_message_types, &mut *tx)
            .await?;
        self.unnest_insert(prepared_set.species_fao, &mut *tx)
            .await?;
        self.unnest_insert(prepared_set.species_fiskeridir, &mut *tx)
            .await?;
        self.unnest_insert(prepared_set.municipalities, &mut *tx)
            .await?;
        self.unnest_insert(prepared_set.counties, &mut *tx).await?;
        self.unnest_insert(prepared_set.vessels, &mut *tx).await?;

        self.add_ers_tra(prepared_set.ers_tra, &mut tx).await?;

        self.unnest_insert(prepared_set.catches, &mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    async fn add_ers_tra<'a>(
        &'a self,
        mut ers_tra: Vec<NewErsTra<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let to_insert = self.ers_tra_to_insert(&ers_tra, tx).await?;
        ers_tra.retain(|e| to_insert.contains(&e.message_id));

        let event_ids = self
            .unnest_insert_returning(ers_tra, &mut **tx)
            .await?
            .into_iter()
            .filter_map(|r| r.vessel_event_id)
            .collect();

        self.connect_trip_to_events(event_ids, VesselEventType::ErsTra, tx)
            .await?;

        Ok(())
    }

    async fn ers_tra_to_insert<'a>(
        &'a self,
        ers_tra: &[NewErsTra<'_>],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashSet<i64>> {
        let message_ids = ers_tra.iter().map(|e| e.message_id).collect::<Vec<_>>();

        let ids = sqlx::query!(
            r#"
SELECT
    u.message_id AS "message_id!"
FROM
    UNNEST($1::BIGINT[]) u (message_id)
    LEFT JOIN ers_tra e ON u.message_id = e.message_id
WHERE
    e.message_id IS NULL
            "#,
            &message_ids,
        )
        .fetch(&mut **tx)
        .map_ok(|r| r.message_id)
        .try_collect::<HashSet<_>>()
        .await?;

        Ok(ids)
    }
}
