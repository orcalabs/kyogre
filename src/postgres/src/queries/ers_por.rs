use crate::{
    error::PostgresError,
    ers_por_set::ErsPorSet,
    models::{Arrival, NewErsPor, NewErsPorCatch},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{ArrivalFilter, FiskeridirVesselId};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_ers_por_set(&self, set: ErsPorSet) -> Result<(), PostgresError> {
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
        self.add_ports(prepared_set.ports, &mut tx).await?;
        self.add_ers_por(prepared_set.ers_por, &mut tx).await?;

        self.add_ers_por_catches(prepared_set.catches, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_por<'a>(
        &'a self,
        ers_por: Vec<NewErsPor>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsPor::unnest_insert(ers_por, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_ers_por_catches<'a>(
        &self,
        catches: Vec<NewErsPorCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsPorCatch::unnest_insert(catches, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn delete_ers_por_impl(&self, year: u32) -> Result<(), PostgresError> {
        sqlx::query!(
            r#"
DELETE FROM ers_arrivals e
WHERE
    e.relevant_year = $1
            "#,
            year as i32
        )
        .execute(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
        .map(|_| ())
    }

    pub async fn ers_arrivals_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        filter: ArrivalFilter,
    ) -> Result<Vec<Arrival>, PostgresError> {
        let landing_facility = match filter {
            ArrivalFilter::WithLandingFacility => Some(true),
            ArrivalFilter::All => None,
        };
        sqlx::query_as!(
            Arrival,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    arrival_timestamp AS "timestamp",
    port_id
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
            vessel_id.0,
            start,
            landing_facility,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
