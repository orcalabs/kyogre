use std::collections::HashSet;

use crate::{
    error::PostgresError,
    ers_tra_set::ErsTraSet,
    models::{NewErsTra, NewErsTraCatch},
    PostgresAdapter,
};
use error_stack::{Result, ResultExt};
use futures::TryStreamExt;
use kyogre_core::VesselEventType;
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

impl PostgresAdapter {
    pub(crate) async fn add_ers_tra_set(&self, set: ErsTraSet) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();

        let mut tx = self.begin().await?;

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

        self.add_ers_tra(prepared_set.ers_tra, &mut tx).await?;

        self.add_ers_tra_catches(prepared_set.catches, &mut tx)
            .await?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_tra<'a>(
        &'a self,
        mut ers_tra: Vec<NewErsTra>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let to_insert = self.ers_tra_to_insert(&ers_tra, tx).await?;
        ers_tra.retain(|e| to_insert.contains(&e.message_id));

        let event_ids = NewErsTra::unnest_insert_returning(ers_tra, &mut **tx)
            .await
            .change_context(PostgresError::Query)?
            .into_iter()
            .filter_map(|r| r.vessel_event_id)
            .collect();

        self.connect_trip_to_events(event_ids, VesselEventType::ErsTra, tx)
            .await?;

        Ok(())
    }

    async fn ers_tra_to_insert<'a>(
        &'a self,
        ers_tra: &[NewErsTra],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashSet<i64>, PostgresError> {
        let message_ids = ers_tra.iter().map(|e| e.message_id).collect::<Vec<_>>();

        sqlx::query!(
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
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_ers_tra_catches<'a>(
        &self,
        catches: Vec<NewErsTraCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsTraCatch::unnest_insert(catches, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
