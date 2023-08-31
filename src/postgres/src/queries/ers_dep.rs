use std::{cmp::min, collections::HashMap};

use crate::{
    error::PostgresError,
    ers_dep_set::ErsDepSet,
    models::{Departure, NewErsDep, NewErsDepCatch, TripAssemblerConflict},
    PostgresAdapter,
};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{FiskeridirVesselId, TripAssemblerId, VesselEventType};
use unnest_insert::{UnnestInsert, UnnestInsertReturning};

impl PostgresAdapter {
    pub(crate) async fn add_ers_dep_set(&self, set: ErsDepSet) -> Result<(), PostgresError> {
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
        self.add_ers_dep(prepared_set.ers_dep, &mut tx).await?;
        self.add_ers_dep_catches(prepared_set.catches, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    async fn add_ers_dep<'a>(
        &'a self,
        ers_dep: Vec<NewErsDep>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let fiskeridir_vessel_ids = ers_dep
            .iter()
            .filter_map(|e| e.fiskeridir_vessel_id)
            .collect::<Vec<_>>();

        sqlx::query!(
            r#"
UPDATE fiskeridir_vessels
SET
    preferred_trip_assembler = $1
WHERE
    fiskeridir_vessel_id = ANY ($2::BIGINT[])
    AND fiskeridir_vessel_id IS NOT NULL
            "#,
            TripAssemblerId::Ers as i32,
            fiskeridir_vessel_ids.as_slice() as _,
        )
        .execute(&mut **tx)
        .await
        .into_report()
        .change_context(PostgresError::Query)?;

        let inserted = NewErsDep::unnest_insert_returning(ers_dep, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)?;

        let len = inserted.len();
        let mut conflicts = HashMap::<i64, TripAssemblerConflict>::with_capacity(len);
        let mut event_ids = Vec::with_capacity(len);

        for i in inserted {
            if let (Some(id), Some(event_id)) = (i.fiskeridir_vessel_id, i.vessel_event_id) {
                conflicts
                    .entry(id)
                    .and_modify(|v| v.timestamp = min(v.timestamp, i.departure_timestamp))
                    .or_insert_with(|| TripAssemblerConflict {
                        fiskeridir_vessel_id: id,
                        timestamp: i.departure_timestamp,
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

    pub(crate) async fn add_ers_dep_catches<'a>(
        &self,
        catches: Vec<NewErsDepCatch>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewErsDepCatch::unnest_insert(catches, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub async fn ers_departures_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
    ) -> Result<Vec<Departure>, PostgresError> {
        sqlx::query_as!(
            Departure,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    departure_timestamp AS "timestamp",
    port_id
FROM
    ers_departures
WHERE
    fiskeridir_vessel_id = $1
    AND departure_timestamp >= GREATEST($2, '1970-01-01T00:00:00Z'::TIMESTAMPTZ)
            "#,
            vessel_id.0,
            start,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
