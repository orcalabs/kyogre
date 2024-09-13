use std::collections::{hash_map::Entry, HashMap};

use crate::{
    error::Result,
    models::{
        AquaCultureEntry, AquaCultureSpecies, AquaCultureTill, DeliveryPoint, ManualDeliveryPoint,
        MattilsynetDeliveryPoint, NewDeliveryPointId, NewSpeciesFiskeridir,
    },
    PostgresAdapter,
};
use fiskeridir_rs::DeliveryPointId;
use futures::{Stream, TryStreamExt};
use kyogre_core::{DateRange, FiskeridirVesselId};
use sqlx::postgres::types::PgRange;
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn delivery_point_impl(
        &self,
        id: &DeliveryPointId,
    ) -> Result<Option<DeliveryPoint>> {
        // Coalesce on delivery_point_id is needed due to a bug in sqlx prepare
        // which flips the nullability on each run
        let dp = sqlx::query_as!(
            DeliveryPoint,
            r#"
SELECT
    COALESCE(d.delivery_point_id, d.delivery_point_id) AS "delivery_point_id!",
    COALESCE(m.name, a.name, mt.name) AS NAME,
    COALESCE(m.address, a.address, mt.address) AS address,
    COALESCE(m.latitude, a.latitude) AS latitude,
    COALESCE(m.longitude, a.longitude) AS longitude
FROM
    delivery_point_ids d
    LEFT JOIN manual_delivery_points m ON m.delivery_point_id = d.delivery_point_id
    LEFT JOIN aqua_culture_register a ON a.delivery_point_id = d.delivery_point_id
    LEFT JOIN mattilsynet_delivery_points mt ON mt.delivery_point_id = d.delivery_point_id
WHERE
    d.delivery_point_id = $1
            "#,
            id.as_ref()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(dp)
    }

    pub(crate) async fn add_deprecated_delivery_point_impl(
        &self,
        old: DeliveryPointId,
        new: DeliveryPointId,
    ) -> Result<()> {
        sqlx::query!(
            r#"
INSERT INTO
    deprecated_delivery_points (old_delivery_point_id, new_delivery_point_id)
VALUES
    ($1, $2)
            "#,
            old.into_inner(),
            new.into_inner(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn delivery_points_log_impl(&self) -> Result<Vec<serde_json::Value>> {
        Ok(sqlx::query!(
            r#"
SELECT
    TO_JSONB(d.*) AS "json!"
FROM
    delivery_points_log d
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| r.json)
        .collect())
    }

    pub(crate) async fn add_manual_delivery_points_impl(
        &self,
        values: Vec<ManualDeliveryPoint>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let ids = values
            .iter()
            .map(|v| NewDeliveryPointId {
                delivery_point_id: v.delivery_point_id.as_ref(),
            })
            .collect();
        self.add_delivery_point_ids(ids, &mut tx).await?;

        ManualDeliveryPoint::unnest_insert(values, &mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) fn delivery_points_impl(&self) -> impl Stream<Item = Result<DeliveryPoint>> + '_ {
        // Coalesce on delivery_point_id is needed due to a bug in sqlx prepare
        // which flips the nullability on each run
        sqlx::query_as!(
            DeliveryPoint,
            r#"
SELECT
    COALESCE(d.delivery_point_id, d.delivery_point_id) AS "delivery_point_id!",
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
        .map_err(From::from)
    }

    pub(crate) async fn add_delivery_point_ids<'a>(
        &'a self,
        delivery_point_ids: Vec<NewDeliveryPointId<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        NewDeliveryPointId::unnest_insert(delivery_point_ids, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_aqua_culture_register_impl(
        &self,
        f_entries: Vec<fiskeridir_rs::AquaCultureEntry>,
    ) -> Result<()> {
        let len = f_entries.len();
        let mut entries = HashMap::with_capacity(len);
        let mut species = HashMap::with_capacity(len);
        let mut aqua_species = HashMap::with_capacity(len);
        let mut tills = HashMap::with_capacity(len);
        let mut ids = Vec::with_capacity(len);

        for a in &f_entries {
            ids.push((&a.delivery_point_id).into());

            species.entry(a.species_code).or_insert_with(|| {
                NewSpeciesFiskeridir::new(a.species_code as i32, Some(&a.species))
            });

            if let Entry::Vacant(e) =
                aqua_species.entry((a.till_nr.as_ref(), a.till_unit.as_ref(), a.species_code))
            {
                e.insert(a.into());
            }

            if let Entry::Vacant(e) = tills.entry((&a.delivery_point_id, a.till_nr.as_ref())) {
                e.insert(a.into());
            }

            entries.insert(&a.delivery_point_id, a.into());
        }

        let values = entries.into_values().collect();
        let species = species.into_values().collect();
        let tills = tills.into_values().collect();
        let aqua_species = aqua_species.into_values().collect();

        let mut tx = self.pool.begin().await?;

        self.add_delivery_point_ids(ids, &mut tx).await?;
        self.add_species_fiskeridir(species, &mut tx).await?;

        AquaCultureEntry::unnest_insert(values, &mut *tx).await?;

        self.add_aqua_culture_register_tills(tills, &mut tx).await?;
        self.add_aqua_culture_register_species(aqua_species, &mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_aqua_culture_register_tills<'a>(
        &'a self,
        tills: Vec<AquaCultureTill<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        AquaCultureTill::unnest_insert(tills, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_aqua_culture_register_species<'a>(
        &'a self,
        species: Vec<AquaCultureSpecies<'_>>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        AquaCultureSpecies::unnest_insert(species, &mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn add_mattilsynet_delivery_points_impl(
        &self,
        delivery_points: Vec<kyogre_core::MattilsynetDeliveryPoint>,
    ) -> Result<()> {
        let len = delivery_points.len();
        let mut items = Vec::with_capacity(len);
        let mut ids = Vec::with_capacity(len);

        for d in &delivery_points {
            ids.push((&d.id).into());
            items.push(d.into());
        }

        let mut tx = self.pool.begin().await?;

        self.add_delivery_point_ids(ids, &mut tx).await?;

        MattilsynetDeliveryPoint::unnest_insert(items, &mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn delivery_points_associated_with_trip_impl(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_landing_coverage: &DateRange,
    ) -> Result<Vec<DeliveryPoint>> {
        let pg_range = PgRange::from(trip_landing_coverage);

        let dps = sqlx::query_as!(
            DeliveryPoint,
            r#"
SELECT
    d.delivery_point_id AS "delivery_point_id!",
    COALESCE(m.name, a.name, mt.name) AS NAME,
    COALESCE(m.address, a.address, mt.address) AS address,
    COALESCE(m.latitude, a.latitude) AS latitude,
    COALESCE(m.longitude, a.longitude) AS longitude
FROM
    landings l
    INNER JOIN delivery_point_ids d ON l.delivery_point_id = d.delivery_point_id
    LEFT JOIN manual_delivery_points m ON m.delivery_point_id = d.delivery_point_id
    LEFT JOIN aqua_culture_register a ON a.delivery_point_id = d.delivery_point_id
    LEFT JOIN mattilsynet_delivery_points mt ON mt.delivery_point_id = d.delivery_point_id
WHERE
    l.fiskeridir_vessel_id = $1
    AND l.landing_timestamp <@ $2::tstzrange
            "#,
            vessel_id.into_inner(),
            pg_range
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(dps)
    }
}
