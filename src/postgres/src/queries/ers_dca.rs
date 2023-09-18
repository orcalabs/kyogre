use std::{cmp::min, collections::HashSet};

use crate::{
    error::PostgresError,
    ers_dca_set::ErsDcaSet,
    models::{NewErsDca, NewErsDcaBody, NewHerringPopulation},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use futures::TryStreamExt;
use kyogre_core::VesselEventType;
use tracing::{event, Level};
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

static CHUNK_SIZE: usize = 100_000;

impl PostgresAdapter {
    pub(crate) async fn add_ers_dca_impl(
        &self,
        ers_dca: Box<
            dyn Iterator<Item = Result<fiskeridir_rs::ErsDca, fiskeridir_rs::Error>> + Send + Sync,
        >,
    ) -> Result<(), PostgresError> {
        let mut tx = self.begin().await?;

        let mut earliest_dca = Utc::now();
        let mut inserted_message_ids = HashSet::new();
        let mut vessel_event_ids = Vec::new();

        let mut chunk = Vec::with_capacity(CHUNK_SIZE);
        for (i, item) in ers_dca.enumerate() {
            match item {
                Err(e) => {
                    event!(Level::ERROR, "failed to read data: {:?}", e);
                }
                Ok(item) => {
                    chunk.push(item);
                    if i % CHUNK_SIZE == 0 && i > 0 {
                        let set = ErsDcaSet::new(chunk.drain(0..))?;
                        self.add_ers_dca_set(
                            set,
                            &mut earliest_dca,
                            &mut inserted_message_ids,
                            &mut vessel_event_ids,
                            &mut tx,
                        )
                        .await?;
                    }
                }
            }
        }
        if !chunk.is_empty() {
            let set = ErsDcaSet::new(chunk.drain(0..))?;
            self.add_ers_dca_set(
                set,
                &mut earliest_dca,
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

        self.update_trips_refresh_boundary(earliest_dca, &mut tx)
            .await?;

        tx.commit().await.change_context(PostgresError::Transaction)
    }

    pub(crate) async fn add_ers_dca_set<'a>(
        &'a self,
        set: ErsDcaSet,
        earliest_dca: &mut DateTime<Utc>,
        inserted_message_ids: &mut HashSet<i64>,
        vessel_event_ids: &mut Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();

        if let Some(ts) = prepared_set
            .ers_dca
            .iter()
            .map(|v| v.message_timestamp)
            .chain(
                prepared_set
                    .ers_dca_bodies
                    .iter()
                    .filter_map(|v| v.start_timestamp),
            )
            .min()
        {
            *earliest_dca = min(*earliest_dca, ts);
        }

        self.add_ers_message_types(prepared_set.ers_message_types, tx)
            .await?;
        self.add_area_groupings(prepared_set.area_groupings, tx)
            .await?;
        self.add_herring_populations(prepared_set.herring_populations, tx)
            .await?;
        self.add_catch_main_areas(prepared_set.main_areas, tx)
            .await?;
        self.add_catch_areas(prepared_set.catch_areas, tx).await?;
        self.add_gear_fao(prepared_set.gear_fao, tx).await?;
        self.add_gear_problems(prepared_set.gear_problems, tx)
            .await?;
        self.add_municipalities(prepared_set.municipalities, tx)
            .await?;
        self.add_economic_zones(prepared_set.economic_zones, tx)
            .await?;
        self.add_counties(prepared_set.counties, tx).await?;
        self.add_fiskeridir_vessels(prepared_set.vessels, tx)
            .await?;
        self.add_ports(prepared_set.ports, tx).await?;
        self.add_species_fao(prepared_set.species_fao, tx).await?;
        self.add_species_fiskeridir(prepared_set.species_fiskeridir, tx)
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
            .filter(|b| inserted_message_ids.contains(&b.message_id))
            .collect();
        self.add_ers_dca_bodies(bodies, tx).await?;

        Ok(())
    }

    async fn add_ers_dca<'a>(
        &'a self,
        mut ers_dca: Vec<NewErsDca>,
        inserted_message_ids: &mut HashSet<i64>,
        vessel_event_ids: &mut Vec<i64>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
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
        .await
        .change_context(PostgresError::Query)?;

        let to_insert = self.ers_dca_to_insert(&message_id, tx).await?;
        ers_dca.retain(|e| to_insert.contains(&e.message_id));

        let inserted = NewErsDca::unnest_insert_returning(ers_dca, &mut **tx)
            .await
            .change_context(PostgresError::Query)?;

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
    ) -> Result<HashSet<i64>, PostgresError> {
        sqlx::query!(
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
        .await
        .change_context(PostgresError::Query)
    }

    async fn add_ers_dca_bodies<'a>(
        &'a self,
        ers_dca_bodies: Vec<NewErsDcaBody>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsDcaBody::unnest_insert(ers_dca_bodies, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_herring_populations<'a>(
        &self,
        herring_populations: Vec<NewHerringPopulation>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewHerringPopulation::unnest_insert(herring_populations, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
