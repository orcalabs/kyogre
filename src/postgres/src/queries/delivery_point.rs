use std::collections::{hash_map::Entry, HashMap};

use crate::{
    error::PostgresError,
    models::{
        AquaCultureEntry, AquaCultureSpecies, AquaCultureTill, DeliveryPoint,
        MattilsynetDeliveryPoint, NewDeliveryPointId, SpeciesFiskeridir,
    },
    PostgresAdapter,
};
use error_stack::{report, IntoReport, Result, ResultExt};
use futures::{Stream, TryStreamExt};
use kyogre_core::TripId;
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) fn delivery_points_impl(
        &self,
    ) -> impl Stream<Item = Result<DeliveryPoint, PostgresError>> + '_ {
        sqlx::query_as!(
            DeliveryPoint,
            r#"
SELECT
    d.delivery_point_id as "delivery_point_id!",
    COALESCE(m.name, a.name, mt.name) AS NAME,
    COALESCE(m.address, a.address, mt.address) AS address,
    COALESCE(m.latitude, a.latitude) AS latitude,
    COALESCE(m.longitude, a.longitude) AS longitude
FROM
    delivery_point_ids d
    LEFT JOIN manual_delivery_points m ON m.delivery_point_id = d.delivery_point_id
    LEFT JOIN aqua_culture_register a ON a.delivery_point_id = d.delivery_point_id
    LEFT JOIN mattilsynet_delivery_points mt ON mt.delivery_point_id = d.delivery_point_id
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn add_delivery_point_ids<'a>(
        &'a self,
        delivery_point_ids: Vec<NewDeliveryPointId>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewDeliveryPointId::unnest_insert(delivery_point_ids, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_aqua_culture_register_impl(
        &self,
        f_entries: Vec<fiskeridir_rs::AquaCultureEntry>,
    ) -> Result<(), PostgresError> {
        let len = f_entries.len();
        let mut entries = HashMap::with_capacity(len);
        let mut species = HashMap::with_capacity(len);
        let mut aqua_species = HashMap::with_capacity(len);
        let mut tills = HashMap::with_capacity(len);
        let mut ids = Vec::with_capacity(len);

        for a in f_entries {
            ids.push(a.delivery_point_id.clone().into());

            species.entry(a.species_code).or_insert_with(|| {
                SpeciesFiskeridir::new(a.species_code as i32, Some(a.species.clone()))
            });

            if let Entry::Vacant(e) =
                aqua_species.entry((a.till_nr.clone(), a.till_unit.clone(), a.species_code))
            {
                e.insert((&a).try_into()?);
            }

            if let Entry::Vacant(e) = tills.entry((a.delivery_point_id.clone(), a.till_nr.clone()))
            {
                e.insert((&a).try_into()?);
            }

            entries.insert(a.delivery_point_id.clone(), a.try_into()?);
        }

        let values = entries.into_values().collect();
        let species = species.into_values().collect();
        let tills = tills.into_values().collect();
        let aqua_species = aqua_species.into_values().collect();

        let mut tx = self.begin().await?;

        self.add_delivery_point_ids(ids, &mut tx).await?;
        self.add_species_fiskeridir(species, &mut tx).await?;

        AquaCultureEntry::unnest_insert(values, &mut *tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)?;

        self.add_aqua_culture_register_tills(tills, &mut tx).await?;
        self.add_aqua_culture_register_species(aqua_species, &mut tx)
            .await?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Query)
    }

    pub(crate) async fn add_aqua_culture_register_tills<'a>(
        &'a self,
        tills: Vec<AquaCultureTill>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        AquaCultureTill::unnest_insert(tills, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_aqua_culture_register_species<'a>(
        &'a self,
        species: Vec<AquaCultureSpecies>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        AquaCultureSpecies::unnest_insert(species, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_mattilsynet_delivery_points_impl(
        &self,
        delivery_points: Vec<kyogre_core::MattilsynetDeliveryPoint>,
    ) -> Result<(), PostgresError> {
        let len = delivery_points.len();
        let mut items = Vec::with_capacity(len);
        let mut ids = Vec::with_capacity(len);

        for d in delivery_points {
            ids.push(d.id.clone().into());
            items.push(d.into());
        }

        let mut tx = self.begin().await?;

        self.add_delivery_point_ids(ids, &mut tx).await?;

        MattilsynetDeliveryPoint::unnest_insert(items, &mut *tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)?;

        tx.commit()
            .await
            .into_report()
            .change_context(PostgresError::Query)
    }

    pub(crate) async fn delivery_points_associated_with_trip_impl(
        &self,
        _trip_id: TripId,
    ) -> Result<Vec<kyogre_core::DeliveryPoint>, PostgresError> {
        // TODO: implement when we have coordinates for delivery points
        Ok(vec![])
    }
}
