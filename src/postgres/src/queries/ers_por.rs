use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use chrono::{DateTime, Utc};
use futures::{future::ready, TryStreamExt};
use kyogre_core::{
    Arrival, ArrivalFilter, BoxIterator, FiskeridirVesselId, TripAssemblerId, VesselEventType,
};
use tracing::error;

use crate::{
    chunk::Chunks,
    error::Result,
    ers_por_set::ErsPorSet,
    models::{NewErsPor, NewTripAssemblerConflict},
    PostgresAdapter,
};

static CHUNK_SIZE: usize = 10_000;

impl PostgresAdapter {
    pub(crate) async fn add_ers_por_impl(
        &self,
        ers_por: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsPor>>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let mut set = ErsPorSet::with_capacity(CHUNK_SIZE);

        let mut chunks = ers_por.chunks(CHUNK_SIZE);
        while let Some(chunk) = chunks.next() {
            // SAFETY: This `transmute` is necessary to "reset" the lifetime of the set so
            // that it no longer borrows from `chunk` at the end of the scope.
            // This is safe as long as the set is completely cleared before `chunk` is
            // dropped.
            let temp_set: &mut ErsPorSet<'_> = unsafe { std::mem::transmute(&mut set) };

            temp_set.add_all(chunk.iter().filter_map(
                |v: &fiskeridir_rs::Result<fiskeridir_rs::ErsPor>| match v {
                    Ok(v) => Some(v),
                    Err(e) => {
                        error!("failed to read data: {e:?}");
                        None
                    }
                },
            ))?;

            self.add_ers_por_set(temp_set, &mut tx).await?;

            temp_set.assert_is_empty();
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_ers_por_set<'a>(
        &'a self,
        set: &mut ErsPorSet<'_>,
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

        self.add_ers_por(set.ers_por(), tx).await?;

        self.unnest_insert(set.catches(), &mut **tx).await?;

        Ok(())
    }

    async fn add_ers_por<'a>(
        &'a self,
        mut ers_por: Vec<NewErsPor<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let to_insert = self.ers_por_to_insert(&ers_por, tx).await?;
        ers_por.retain(|e| to_insert.contains(&e.message_id));

        let len = ers_por.len();
        let mut conflicts =
            HashMap::<FiskeridirVesselId, NewTripAssemblerConflict>::with_capacity(len);
        let mut event_ids = Vec::with_capacity(len);

        self.unnest_insert_returning(ers_por, &mut **tx)
            .try_for_each(|i| {
                if let (Some(id), Some(event_id)) = (i.fiskeridir_vessel_id, i.vessel_event_id) {
                    conflicts
                        .entry(id)
                        .and_modify(|v| v.timestamp = min(v.timestamp, i.arrival_timestamp))
                        .or_insert_with(|| NewTripAssemblerConflict {
                            fiskeridir_vessel_id: id,
                            timestamp: i.arrival_timestamp,
                            vessel_event_id: Some(event_id),
                            event_type: VesselEventType::ErsPor,
                            vessel_event_timestamp: i.arrival_timestamp,
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

    async fn ers_por_to_insert<'a>(
        &'a self,
        ers_por: &[NewErsPor<'_>],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashSet<i64>> {
        let message_ids = ers_por.iter().map(|e| e.message_id).collect::<Vec<_>>();

        let ids = sqlx::query!(
            r#"
SELECT
    u.message_id AS "message_id!"
FROM
    UNNEST($1::BIGINT[]) u (message_id)
    LEFT JOIN ers_arrivals e ON u.message_id = e.message_id
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

    pub async fn ers_arrivals_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        filter: ArrivalFilter,
    ) -> Result<Vec<Arrival>> {
        let landing_facility = match filter {
            ArrivalFilter::WithLandingFacility => Some(true),
            ArrivalFilter::All => None,
        };
        let arrivals = sqlx::query_as!(
            Arrival,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    arrival_timestamp AS "timestamp",
    port_id,
    message_number
FROM
    ers_arrivals
WHERE
    fiskeridir_vessel_id = $1
    AND arrival_timestamp >= GREATEST($2, '1970-01-01T00:00:00Z'::TIMESTAMPTZ)
    AND (
        $3::bool IS NULL
        OR landing_facility IS NOT NULL
    )
            "#,
            vessel_id.into_inner(),
            start,
            landing_facility,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(arrivals)
    }
}
