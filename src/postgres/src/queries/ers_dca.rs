use std::collections::HashSet;

use futures::{future::ready, TryStreamExt};
use kyogre_core::{BoxIterator, VesselEventType};
use tracing::error;

use crate::{
    chunk::Chunks, error::Result, ers_dca_set::ErsDcaSet, models::NewErsDca, PostgresAdapter,
};

static CHUNK_SIZE: usize = 100_000;

impl PostgresAdapter {
    pub(crate) async fn add_ers_dca_impl(
        &self,
        ers_dca: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsDca>>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let mut inserted_message_ids = HashSet::new();
        let mut vessel_event_ids = Vec::new();

        let mut set = ErsDcaSet::with_capacity(CHUNK_SIZE);

        let mut chunks = ers_dca.chunks(CHUNK_SIZE);
        while let Some(chunk) = chunks.next() {
            // SAFETY: This `transmute` is necessary to "reset" the lifetime of the set so
            // that it no longer borrows from `chunk` at the end of the scope.
            // This is safe as long as the set is completely cleared before `chunk` is
            // dropped.
            let temp_set: &mut ErsDcaSet<'_> = unsafe { std::mem::transmute(&mut set) };

            temp_set.add_all(chunk.iter().filter_map(
                |v: &fiskeridir_rs::Result<fiskeridir_rs::ErsDca>| match v {
                    Ok(v) => Some(v),
                    Err(e) => {
                        error!("failed to read data: {e:?}");
                        None
                    }
                },
            ))?;

            self.add_ers_dca_set(
                temp_set,
                &mut inserted_message_ids,
                &mut vessel_event_ids,
                &mut tx,
            )
            .await?;

            temp_set.assert_is_empty();
        }

        self.connect_trip_to_events(&vessel_event_ids, VesselEventType::ErsDca, &mut tx)
            .await?;

        let message_ids = inserted_message_ids.into_iter().collect::<Vec<_>>();

        let haul_vessel_event_ids = self.add_hauls(&message_ids, &mut tx).await?;
        self.update_trip_position_cargo_weight_distribution_status(&haul_vessel_event_ids, &mut tx)
            .await?;
        self.add_hauls_matrix(&message_ids, &mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_ers_dca_set<'a>(
        &'a self,
        set: &mut ErsDcaSet<'_>,
        inserted_message_ids: &mut HashSet<i64>,
        vessel_event_ids: &mut Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        self.unnest_insert(set.ers_message_types(), &mut **tx)
            .await?;
        self.unnest_insert(set.area_groupings(), &mut **tx).await?;
        self.unnest_insert(set.herring_populations(), &mut **tx)
            .await?;
        self.unnest_insert(set.main_areas(), &mut **tx).await?;
        self.unnest_insert(set.catch_areas(), &mut **tx).await?;
        self.unnest_insert(set.gear_fao(), &mut **tx).await?;
        self.unnest_insert(set.gear_problems(), &mut **tx).await?;
        self.unnest_insert(set.municipalities(), &mut **tx).await?;
        self.unnest_insert(set.economic_zones(), &mut **tx).await?;
        self.unnest_insert(set.counties(), &mut **tx).await?;
        self.unnest_insert(set.vessels(), &mut **tx).await?;
        self.unnest_insert(set.ports(), &mut **tx).await?;
        self.unnest_insert(set.species_fao(), &mut **tx).await?;
        self.unnest_insert(set.species_fiskeridir(), &mut **tx)
            .await?;

        self.add_ers_dca(set.ers_dca(), inserted_message_ids, vessel_event_ids, tx)
            .await?;

        let bodies = set
            .ers_dca_bodies()
            .filter(|b| inserted_message_ids.contains(&b.message_id));

        self.unnest_insert(bodies, &mut **tx).await?;

        Ok(())
    }

    async fn add_ers_dca<'a>(
        &'a self,
        mut ers_dca: Vec<NewErsDca<'_>>,
        inserted_message_ids: &mut HashSet<i64>,
        vessel_event_ids: &mut Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let len = ers_dca.len();
        let mut message_id = Vec::with_capacity(len);
        let mut message_version = Vec::with_capacity(len);

        for e in ers_dca.iter() {
            message_id.push(e.message_id);
            message_version.push(e.message_version);
        }

        sqlx::query!(
            r#"
DELETE FROM ers_dca e USING UNNEST($1::BIGINT[], $2::INT[]) u (message_id, message_version)
WHERE
    e.message_id = u.message_id
    AND e.message_version < u.message_version
            "#,
            &message_id,
            &message_version,
        )
        .execute(&mut **tx)
        .await?;

        let to_insert = self.ers_dca_to_insert(&message_id, tx).await?;
        ers_dca.retain(|e| to_insert.contains(&e.message_id));

        self.unnest_insert_returning(ers_dca, &mut **tx)
            .try_for_each(|i| {
                inserted_message_ids.insert(i.message_id);
                if let Some(event_id) = i.vessel_event_id {
                    vessel_event_ids.push(event_id);
                }
                ready(Ok(()))
            })
            .await?;

        Ok(())
    }

    async fn ers_dca_to_insert<'a>(
        &'a self,
        message_ids: &[i64],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashSet<i64>> {
        let ids = sqlx::query!(
            r#"
SELECT
    u.message_id AS "message_id!"
FROM
    UNNEST($1::BIGINT[]) u (message_id)
    LEFT JOIN ers_dca e ON u.message_id = e.message_id
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
