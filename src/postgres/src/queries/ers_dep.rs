use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use futures::{TryStreamExt, future::ready};
use kyogre_core::{
    BoxIterator, DateRange, DepartureWeight, FiskeridirVesselId, TripAssemblerId, VesselEventType,
};
use tracing::error;

use crate::{
    PostgresAdapter,
    chunk::Chunks,
    error::Result,
    ers_dep_set::ErsDepSet,
    models::{NewErsDep, NewTripAssemblerConflict},
};

static CHUNK_SIZE: usize = 10_000;

impl PostgresAdapter {
    pub(crate) async fn add_ers_dep_impl(
        &self,
        ers_dep: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsDep>>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let mut set = ErsDepSet::with_capacity(CHUNK_SIZE);

        let mut chunks = ers_dep.chunks(CHUNK_SIZE);
        while let Some(chunk) = chunks.next() {
            // SAFETY: This `transmute` is necessary to "reset" the lifetime of the set so
            // that it no longer borrows from `chunk` at the end of the scope.
            // This is safe as long as the set is completely cleared before `chunk` is
            // dropped.
            let temp_set: &mut ErsDepSet<'_> = unsafe { std::mem::transmute(&mut set) };

            temp_set.add_all(chunk.iter().filter_map(
                |v: &fiskeridir_rs::Result<fiskeridir_rs::ErsDep>| match v {
                    Ok(v) => Some(v),
                    Err(e) => {
                        error!("failed to read data: {e:?}");
                        None
                    }
                },
            ))?;

            self.add_ers_dep_set(temp_set, &mut tx).await?;

            temp_set.assert_is_empty();
        }

        tx.commit().await?;

        Ok(())
    }

    async fn add_ers_dep_set<'a>(
        &'a self,
        set: &mut ErsDepSet<'_>,
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
        self.unnest_insert(set.ports(), &mut **tx).await?;

        self.add_ers_dep(set.ers_dep(), tx).await?;

        self.unnest_insert(set.catches(), &mut **tx).await?;

        Ok(())
    }

    async fn add_ers_dep<'a>(
        &'a self,
        mut ers_dep: Vec<NewErsDep<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let to_insert = self.ers_dep_to_insert(&ers_dep, tx).await?;
        ers_dep.retain(|e| to_insert.contains(&e.message_id));

        let len = ers_dep.len();
        let mut conflicts =
            HashMap::<FiskeridirVesselId, NewTripAssemblerConflict>::with_capacity(len);
        let mut event_ids = Vec::with_capacity(len);

        self.unnest_insert_returning(ers_dep, &mut **tx)
            .try_for_each(|i| {
                if let (Some(id), Some(event_id)) = (i.fiskeridir_vessel_id, i.vessel_event_id) {
                    conflicts
                        .entry(id)
                        .and_modify(|v| v.timestamp = min(v.timestamp, i.departure_timestamp))
                        .or_insert_with(|| NewTripAssemblerConflict {
                            fiskeridir_vessel_id: id,
                            timestamp: i.departure_timestamp,
                            vessel_event_id: Some(event_id),
                            event_type: VesselEventType::ErsDep,
                            vessel_event_timestamp: i.departure_timestamp,
                        });
                    event_ids.push(event_id);
                }
                ready(Ok(()))
            })
            .await?;

        self.add_trip_assembler_conflicts(
            conflicts.into_values().collect(),
            TripAssemblerId::Ers,
            tx,
        )
        .await?;

        Ok(())
    }

    async fn ers_dep_to_insert<'a>(
        &'a self,
        ers_dep: &[NewErsDep<'_>],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashSet<i64>> {
        let message_ids = ers_dep.iter().map(|e| e.message_id).collect::<Vec<_>>();

        let ids = sqlx::query!(
            r#"
SELECT
    u.message_id AS "message_id!"
FROM
    UNNEST($1::BIGINT[]) u (message_id)
    LEFT JOIN ers_departures e ON u.message_id = e.message_id
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

    pub(crate) async fn departure_weights_from_range_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> Result<Vec<DepartureWeight>> {
        sqlx::query_as!(
            DepartureWeight,
            r#"
WITH
    deps AS (
        SELECT
            MAX(message_id) AS message_id,
            departure_timestamp
        FROM
            ers_departures
        WHERE
            fiskeridir_vessel_id = $1::BIGINT
            AND departure_timestamp >= $2::TIMESTAMPTZ
            AND departure_timestamp < $3::TIMESTAMPTZ
        GROUP BY
            departure_timestamp
    )
SELECT
    e.departure_timestamp,
    COALESCE(SUM(c.living_weight), 0)::DOUBLE PRECISION AS "weight!"
FROM
    deps e
    LEFT JOIN ers_departure_catches c ON e.message_id = c.message_id
GROUP BY
    e.message_id,
    e.departure_timestamp
ORDER BY
    e.departure_timestamp ASC
            "#,
            vessel_id.into_inner(),
            range.start(),
            range.end(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.into())
    }
}
