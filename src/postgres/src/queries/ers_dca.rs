use crate::{
    error::PostgresError,
    ers_dca_set::ErsDcaSet,
    models::{
        NewErsDca, NewErsDcaCatch, NewErsDcaOther, NewErsDcaWhaleCatch, NewHerringPopulation,
    },
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_ers_dca_set(&self, set: ErsDcaSet) -> Result<(), PostgresError> {
        let prepared_set = set.prepare();

        let earliest_dca = prepared_set
            .ers_dca
            .iter()
            .flat_map(|v| [v.message_timestamp, v.start_timestamp])
            .min();

        let mut tx = self.begin().await?;

        self.add_ers_message_types(prepared_set.ers_message_types, &mut tx)
            .await?;
        self.add_area_groupings(prepared_set.area_groupings, &mut tx)
            .await?;
        self.add_herring_populations(prepared_set.herring_populations, &mut tx)
            .await?;
        self.add_catch_main_areas(prepared_set.main_areas, &mut tx)
            .await?;
        self.add_catch_areas(prepared_set.catch_areas, &mut tx)
            .await?;
        self.add_gear_fao(prepared_set.gear_fao, &mut tx).await?;
        self.add_gear_problems(prepared_set.gear_problems, &mut tx)
            .await?;
        self.add_municipalities(prepared_set.municipalities, &mut tx)
            .await?;
        self.add_economic_zones(prepared_set.economic_zones, &mut tx)
            .await?;
        self.add_counties(prepared_set.counties, &mut tx).await?;
        self.add_fiskeridir_vessels(prepared_set.vessels, &mut tx)
            .await?;
        self.add_ports(prepared_set.ports, &mut tx).await?;
        self.add_species_fao(prepared_set.species_fao, &mut tx)
            .await?;
        self.add_species_fiskeridir(prepared_set.species_fiskeridir, &mut tx)
            .await?;
        self.add_ers_dca(prepared_set.ers_dca, &mut tx).await?;
        self.add_ers_dca_other(prepared_set.ers_dca_other, &mut tx)
            .await?;

        self.add_ers_dca_catches(prepared_set.catches, &mut tx)
            .await?;
        self.add_ers_dca_whale_catches(prepared_set.whale_catches, &mut tx)
            .await?;

        if let Some(ts) = earliest_dca {
            self.update_trips_refresh_boundary(ts, &mut tx).await?;
        }

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_dca<'a>(
        &'a self,
        ers_dca: Vec<NewErsDca>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsDca::unnest_insert(ers_dca, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    async fn add_ers_dca_other<'a>(
        &'a self,
        ers_dca: Vec<NewErsDcaOther>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsDcaOther::unnest_insert(ers_dca, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    async fn add_ers_dca_catches<'a>(
        &'a self,
        catches: Vec<NewErsDcaCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsDcaCatch::unnest_insert(catches, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    async fn add_ers_dca_whale_catches<'a>(
        &'a self,
        whale_catches: Vec<NewErsDcaWhaleCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsDcaWhaleCatch::unnest_insert(whale_catches, &mut **tx)
            .await
            .into_report()
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
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn delete_ers_dca_impl(&self, year: u32) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
DELETE FROM ers_dca
WHERE
    relevant_year = $1
            "#,
            year as i32
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        Ok(())
    }
}
