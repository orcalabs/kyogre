use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use crate::{
    error::Result,
    ers_dep_set::ErsDepSet,
    models::{NewErsDep, NewErsDepCatch, NewTripAssemblerConflict},
    PostgresAdapter,
};
use futures::TryStreamExt;
use kyogre_core::{Departure, FiskeridirVesselId, TripAssemblerId, VesselEventType};
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

impl PostgresAdapter {
    pub(crate) async fn add_ers_dep_set(&self, set: ErsDepSet) -> Result<()> {
        let prepared_set = set.prepare();

        let mut tx = self.pool.begin().await?;

        self.add_ers_message_types(prepared_set.ers_message_types, &mut tx)
            .await?;
        self.add_species_fao(prepared_set.species_fao, &mut tx)
            .await?;
        self.add_species_fiskeridir(prepared_set.species_fiskeridir, &mut tx)
            .await?;
        self.add_municipalities(prepared_set.municipalities, &mut tx)
            .await?;
        self.add_counties(prepared_set.counties, &mut tx).await?;
        self.add_fiskeridir_vessels(prepared_set.vessels, &mut tx)
            .await?;
        self.add_ports(prepared_set.ports, &mut tx).await?;
        self.add_ers_dep(prepared_set.ers_dep, &mut tx).await?;
        self.add_ers_dep_catches(prepared_set.catches, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn add_ers_dep<'a>(
        &'a self,
        mut ers_dep: Vec<NewErsDep>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let to_insert = self.ers_dep_to_insert(&ers_dep, tx).await?;
        ers_dep.retain(|e| to_insert.contains(&e.message_id));

        let inserted = NewErsDep::unnest_insert_returning(ers_dep, &mut **tx).await?;

        let len = inserted.len();
        let mut conflicts =
            HashMap::<FiskeridirVesselId, NewTripAssemblerConflict>::with_capacity(len);
        let mut event_ids = Vec::with_capacity(len);

        for i in inserted {
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
        }

        self.add_trip_assembler_conflicts(
            conflicts.into_values().collect(),
            TripAssemblerId::Ers,
            tx,
        )
        .await?;
        self.connect_trip_to_events(event_ids, VesselEventType::ErsDep, tx)
            .await?;

        Ok(())
    }

    async fn ers_dep_to_insert<'a>(
        &'a self,
        ers_dep: &[NewErsDep],
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

    pub(crate) async fn add_ers_dep_catches<'a>(
        &self,
        catches: Vec<NewErsDepCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewErsDepCatch::unnest_insert(catches, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn all_ers_departures_impl(&self) -> Result<Vec<Departure>> {
        let dep = sqlx::query_as!(
            Departure,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    departure_timestamp AS "timestamp",
    port_id
FROM
    ers_departures
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(dep)
    }
}
