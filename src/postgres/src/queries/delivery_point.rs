use std::collections::{hash_map::Entry, HashMap};

use fiskeridir_rs::DeliveryPointId;
use futures::{Stream, TryStreamExt};
use kyogre_core::{DateRange, DeliveryPoint, FiskeridirVesselId};
use sqlx::postgres::types::PgRange;

use crate::{
    error::Result,
    models::{
        AquaCultureEntry, AquaCultureSpecies, AquaCultureTill, MattilsynetDeliveryPoint,
        NewDeliveryPointId, NewSpeciesFiskeridir,
    },
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn delivery_points_impl(&self) -> impl Stream<Item = Result<DeliveryPoint>> + '_ {
        self.delivery_points_inner(None)
    }

    pub(crate) fn delivery_points_inner(
        &self,
        id: Option<&DeliveryPointId>,
    ) -> impl Stream<Item = Result<DeliveryPoint>> + '_ {
        // Coalesce on delivery_point_id is needed due to a bug in sqlx prepare
        // which flips the nullability on each run
        sqlx::query_as!(
            DeliveryPoint,
            r#"
SELECT
    COALESCE(d.delivery_point_id, d.delivery_point_id) AS "id!: DeliveryPointId",
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
    (
        $1::TEXT IS NULL
        OR d.delivery_point_id = $1
    )
            "#,
            id as Option<&DeliveryPointId>,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
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
            ids.push(NewDeliveryPointId::from(&a.delivery_point_id));

            species.entry(a.species_code).or_insert_with(|| {
                NewSpeciesFiskeridir::new(a.species_code as i32, Some(&a.species))
            });

            if let Entry::Vacant(e) =
                aqua_species.entry((a.till_nr.as_ref(), a.till_unit.as_ref(), a.species_code))
            {
                e.insert(AquaCultureSpecies::from(a));
            }

            if let Entry::Vacant(e) = tills.entry((&a.delivery_point_id, a.till_nr.as_ref())) {
                e.insert(AquaCultureTill::from(a));
            }

            entries.insert(&a.delivery_point_id, AquaCultureEntry::from(a));
        }

        let values = entries.into_values();
        let species = species.into_values();
        let tills = tills.into_values();
        let aqua_species = aqua_species.into_values();

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(ids, &mut *tx).await?;
        self.unnest_insert(species, &mut *tx).await?;
        self.unnest_insert(values, &mut *tx).await?;
        self.unnest_insert(tills, &mut *tx).await?;
        self.unnest_insert(aqua_species, &mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_mattilsynet_delivery_points_impl(
        &self,
        delivery_points: Vec<kyogre_core::MattilsynetDeliveryPoint>,
    ) -> Result<()> {
        let len = delivery_points.len();
        let mut items = Vec::<MattilsynetDeliveryPoint<'_>>::with_capacity(len);
        let mut ids = Vec::<NewDeliveryPointId<'_>>::with_capacity(len);

        for d in &delivery_points {
            ids.push((&d.id).into());
            items.push(d.into());
        }

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(ids, &mut *tx).await?;
        self.unnest_insert(items, &mut *tx).await?;

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
    d.delivery_point_id AS "id!: DeliveryPointId",
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
