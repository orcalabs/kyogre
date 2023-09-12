use crate::{
    error::PostgresError,
    ers_tra_set::ErsTraSet,
    models::{NewErsTra, NewErsTraCatch},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::VesselEventType;
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

impl PostgresAdapter {
    pub(crate) async fn add_ers_tra_set(&self, set: ErsTraSet) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();

        let earliest_tra = prepared_set
            .ers_tra
            .iter()
            .flat_map(|v| {
                if let Some(ts) = v.reloading_timestamp {
                    vec![v.message_timestamp, ts]
                } else {
                    vec![v.message_timestamp]
                }
            })
            .min();

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

        if let Some(ts) = earliest_tra {
            self.update_trips_refresh_boundary(ts, &mut tx).await?;
        }

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_tra<'a>(
        &'a self,
        ers_tra: Vec<NewErsTra>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let event_ids = NewErsTra::unnest_insert_returning(ers_tra, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)?
            .into_iter()
            .filter_map(|r| r.vessel_event_id)
            .collect();

        self.connect_trip_to_events(event_ids, VesselEventType::ErsTra, tx)
            .await?;

        Ok(())
    }

    pub(crate) async fn add_ers_tra_catches<'a>(
        &self,
        catches: Vec<NewErsTraCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsTraCatch::unnest_insert(catches, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }
}
