use std::collections::HashSet;

use futures::TryStreamExt;
use kyogre_core::VesselEventType;
use tracing::error;

use crate::{error::Result, ers_dca_set::ErsDcaSet, models::NewErsDca, PostgresAdapter};

static CHUNK_SIZE: usize = 100_000;

impl PostgresAdapter {
    pub(crate) async fn add_ers_dca_impl(
        &self,
        ers_dca: Box<
            dyn Iterator<Item = std::result::Result<fiskeridir_rs::ErsDca, fiskeridir_rs::Error>>
                + Send
                + Sync,
        >,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let mut inserted_message_ids = HashSet::new();
        let mut vessel_event_ids = Vec::new();

        let mut chunk = Vec::with_capacity(CHUNK_SIZE);
        for (i, item) in ers_dca.enumerate() {
            match item {
                Err(e) => {
                    error!("failed to read data: {e:?}");
                }
                Ok(item) => {
                    chunk.push(item);
                    if i % CHUNK_SIZE == 0 && i > 0 {
                        let set = ErsDcaSet::new(chunk.iter())?;
                        self.add_ers_dca_set(
                            set,
                            &mut inserted_message_ids,
                            &mut vessel_event_ids,
                            &mut tx,
                        )
                        .await?;
                        chunk.clear();
                    }
                }
            }
        }
        if !chunk.is_empty() {
            let set = ErsDcaSet::new(chunk.iter())?;
            self.add_ers_dca_set(
                set,
                &mut inserted_message_ids,
                &mut vessel_event_ids,
                &mut tx,
            )
            .await?;
        }

        self.connect_trip_to_events(vessel_event_ids, VesselEventType::ErsDca, &mut tx)
            .await?;

        let message_ids = inserted_message_ids.into_iter().collect::<Vec<_>>();

        self.add_hauls(&message_ids, &mut tx).await?;
        self.add_hauls_matrix(&message_ids, &mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_ers_dca_set<'a>(
        &'a self,
        set: ErsDcaSet<'_>,
        inserted_message_ids: &mut HashSet<i64>,
        vessel_event_ids: &mut Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let prepared_set = set.prepare();

        self.unnest_insert(prepared_set.ers_message_types, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.area_groupings, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.herring_populations, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.main_areas, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.catch_areas, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.gear_fao, &mut **tx).await?;
        self.unnest_insert(prepared_set.gear_problems, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.municipalities, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.economic_zones, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.counties, &mut **tx).await?;
        self.unnest_insert(prepared_set.vessels, &mut **tx).await?;
        self.unnest_insert(prepared_set.ports, &mut **tx).await?;
        self.unnest_insert(prepared_set.species_fao, &mut **tx)
            .await?;
        self.unnest_insert(prepared_set.species_fiskeridir, &mut **tx)
            .await?;
        self.add_ers_dca(
            prepared_set.ers_dca,
            inserted_message_ids,
            vessel_event_ids,
            tx,
        )
        .await?;

        let bodies = prepared_set
            .ers_dca_bodies
            .into_iter()
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

        let inserted = self.unnest_insert_returning(ers_dca, &mut **tx).await?;

        for i in inserted {
            inserted_message_ids.insert(i.message_id);
            if let Some(event_id) = i.vessel_event_id {
                vessel_event_ids.push(event_id);
            }
        }

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
