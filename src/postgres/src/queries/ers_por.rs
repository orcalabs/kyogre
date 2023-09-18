use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use crate::{
    error::PostgresError,
    ers_por_set::ErsPorSet,
    models::{Arrival, NewErsPor, NewErsPorCatch, TripAssemblerConflict},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use futures::TryStreamExt;
use kyogre_core::{ArrivalFilter, FiskeridirVesselId, TripAssemblerId, VesselEventType};
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

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
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_por<'a>(
        &'a self,
        mut ers_por: Vec<NewErsPor>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let to_insert = self.ers_por_to_insert(&ers_por, tx).await?;
        ers_por.retain(|e| to_insert.contains(&e.message_id));

        let inserted = NewErsPor::unnest_insert_returning(ers_por, &mut **tx)
            .await
            .change_context(PostgresError::Query)?;

        let len = inserted.len();
        let mut conflicts = HashMap::<i64, TripAssemblerConflict>::with_capacity(len);
        let mut event_ids = Vec::with_capacity(len);

        for i in inserted {
            if let (Some(id), Some(event_id)) = (i.fiskeridir_vessel_id, i.vessel_event_id) {
                conflicts
                    .entry(id)
                    .and_modify(|v| v.timestamp = min(v.timestamp, i.arrival_timestamp))
                    .or_insert_with(|| TripAssemblerConflict {
                        fiskeridir_vessel_id: id,
                        timestamp: i.arrival_timestamp,
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
        self.connect_trip_to_events(event_ids, VesselEventType::ErsPor, tx)
            .await?;

        Ok(())
    }

    async fn ers_por_to_insert<'a>(
        &'a self,
        ers_por: &[NewErsPor],
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<HashSet<i64>, PostgresError> {
        let message_ids = ers_por.iter().map(|e| e.message_id).collect::<Vec<_>>();

        sqlx::query!(
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
        .await
        .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_ers_por_catches<'a>(
        &self,
        catches: Vec<NewErsPorCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsPorCatch::unnest_insert(catches, &mut **tx)
            .await
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
        .change_context(PostgresError::Query)
    }
}
