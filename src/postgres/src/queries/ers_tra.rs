use std::collections::HashSet;

use futures::{future::ready, TryStreamExt};
use kyogre_core::{BoxIterator, VesselEventType};
use tracing::error;

use crate::{
    chunk::Chunks, error::Result, ers_tra_set::ErsTraSet, models::NewErsTra, PostgresAdapter,
};

static CHUNK_SIZE: usize = 10_000;

impl PostgresAdapter {
    pub(crate) async fn add_ers_tra_impl(
        &self,
        ers_tra: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsTra>>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let mut set = ErsTraSet::with_capacity(CHUNK_SIZE);

        let mut chunks = ers_tra.chunks(CHUNK_SIZE);
        while let Some(chunk) = chunks.next() {
            // SAFETY: This `transmute` is necessary to "reset" the lifetime of the set so
            // that it no longer borrows from `chunk` at the end of the scope.
            // This is safe as long as the set is completely cleared before `chunk` is
            // dropped.
            let temp_set: &mut ErsTraSet<'_> = unsafe { std::mem::transmute(&mut set) };

            temp_set.add_all(chunk.iter().filter_map(
                |v: &fiskeridir_rs::Result<fiskeridir_rs::ErsTra>| match v {
                    Ok(v) => Some(v),
                    Err(e) => {
                        error!("failed to read data: {e:?}");
                        None
                    }
                },
            ))?;

            self.add_ers_tra_set(temp_set, &mut tx).await?;

            temp_set.assert_is_empty();
        }

        tx.commit().await?;

        Ok(())
    }

    async fn add_ers_tra_set<'a>(
        &'a self,
        set: &mut ErsTraSet<'_>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        self.unnest_insert(set.ers_message_types(), &mut **tx)
            .await?;
        self.unnest_insert(set.species_fao(), &mut **tx).await?;
        self.unnest_insert(set.species_fiskeridir(), &mut **tx)
            .await?;
        self.unnest_insert(set.municipalities(), &mut **tx).await?;
        self.unnest_insert(set.counties(), &mut **tx).await?;
        self.unnest_insert(set.vessels(), &mut **tx).await?;

        let message_ids = self.add_ers_tra(set.ers_tra(), tx).await?;

        self.unnest_insert(set.catches(), &mut **tx).await?;

        self.add_ers_tra_reloads(&message_ids, tx).await?;

        Ok(())
    }

    async fn add_ers_tra_reloads(
        &self,
        message_ids: &[i64],
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    ers_tra_reloads (
        message_id,
        vessel_event_id,
        vessel_event_type_id,
        message_timestamp,
        reloading_timestamp,
        latitude,
        longitude,
        fiskeridir_vessel_id,
        reload_to,
        reload_from,
        reload_to_call_sign,
        reload_from_call_sign,
        catches
    )
SELECT
    e.message_id,
    e.vessel_event_id,
    CASE
        WHEN e.vessel_event_id IS NULL THEN NULL
        ELSE $2::INT
    END,
    e.message_timestamp,
    e.reloading_timestamp,
    e.start_latitude,
    e.start_longitude,
    e.fiskeridir_vessel_id,
    MAX(v.fiskeridir_vessel_id),
    MAX(v2.fiskeridir_vessel_id),
    e.reload_to_vessel_call_sign,
    e.reload_from_vessel_call_sign,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'living_weight',
                COALESCE(c.living_weight, 0.0),
                'species_group_id',
                c.species_group_id,
                'catch_quantum',
                c.ers_quantum_type_id
            )
        ) FILTER (
            WHERE
                c.message_id IS NOT NULL
        ),
        '[]'
    )
FROM
    UNNEST($1::BIGINT[]) u (message_id)
    INNER JOIN ers_tra e ON u.message_id = e.message_id
    INNER JOIN ers_tra_catches c ON c.message_id = e.message_id
    --! TODO: implement scheme to set this post insert to handle changes in active_vessels
    LEFT JOIN active_vessels v ON v.call_sign = e.reload_to_vessel_call_sign
    LEFT JOIN active_vessels v2 ON v2.call_sign = e.reload_from_vessel_call_sign
GROUP BY
    e.message_id
            "#,
            &message_ids,
            VesselEventType::ErsTra as i32,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn add_ers_tra<'a>(
        &'a self,
        mut ers_tra: Vec<NewErsTra<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<Vec<i64>> {
        let to_insert = self.ers_tra_to_insert(&ers_tra, tx).await?;
        ers_tra.retain(|e| to_insert.contains(&e.message_id));

        let len = ers_tra.len();
        let mut event_ids = Vec::with_capacity(len);
        let mut message_ids = Vec::with_capacity(len);

        self.unnest_insert_returning(ers_tra, &mut **tx)
            .try_for_each(|v| {
                if let Some(event_id) = v.vessel_event_id {
                    event_ids.push(event_id);
                }
                message_ids.push(v.message_id);
                ready(Ok(()))
            })
            .await?;

        self.connect_trip_to_events(&event_ids, VesselEventType::ErsTra, tx)
            .await?;

        Ok(message_ids)
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
