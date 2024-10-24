use std::collections::HashMap;

use fiskeridir_rs::{CallSign, GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, StreamExt, TryStreamExt};
use kyogre_core::{
    ActiveVesselConflict, FiskeridirVesselId, Mmsi, NewVesselConflict, TripAssemblerId,
    VesselSource,
};

use crate::{
    error::Result,
    models::{
        FiskeridirAisVesselCombination, NewMunicipality, NewRegisterVessel, VesselConflictInsert,
    },
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) fn active_vessel_conflicts_impl(
        &self,
    ) -> impl Stream<Item = Result<ActiveVesselConflict>> + '_ {
        sqlx::query_as!(
            ActiveVesselConflict,
            r#"
SELECT
    call_sign AS "call_sign!: CallSign",
    mmsis AS "mmsis!: Vec<Option<Mmsi>>",
    fiskeridir_vessel_ids AS "vessel_ids!: Vec<Option<FiskeridirVesselId>>",
    fiskeridir_vessel_source_ids AS "sources!: Vec<Option<VesselSource>>"
FROM
    fiskeridir_ais_vessel_active_conflicts
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn manual_conflict_override_impl(
        &self,
        overrides: Vec<NewVesselConflict>,
    ) -> Result<()> {
        let mut mmsi = Vec::with_capacity(overrides.len());
        let mut fiskeridir_vessel_id = Vec::with_capacity(overrides.len());

        overrides.iter().for_each(|v| {
            if let Some(val) = v.mmsi {
                mmsi.push(val);
            }
            fiskeridir_vessel_id.push(v.vessel_id);
        });

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
INSERT INTO
    ais_vessels (mmsi)
SELECT
    *
FROM
    UNNEST($1::INT[])
ON CONFLICT DO NOTHING
            "#,
            &mmsi as &[Mmsi],
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    fiskeridir_vessels (fiskeridir_vessel_id)
SELECT
    *
FROM
    UNNEST($1::BIGINT[])
ON CONFLICT DO NOTHING
            "#,
            &fiskeridir_vessel_id as &[FiskeridirVesselId],
        )
        .execute(&mut *tx)
        .await?;

        self.unnest_insert_from::<_, _, VesselConflictInsert>(overrides, &mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn refresh_vessel_mappings<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
DELETE FROM fiskeridir_ais_vessel_mapping_whitelist
WHERE
    is_manual = FALSE;
            "#,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM fiskeridir_ais_vessel_active_conflicts
            "#,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id, mmsi, call_sign)
SELECT
    (ARRAY_AGG(f.fiskeridir_vessel_id)) [1],
    (ARRAY_AGG(a.mmsi)) [1],
    f.call_sign
FROM
    fiskeridir_vessels AS f
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
WHERE
    f.call_sign IS NOT NULL
    AND NOT (f.call_sign = ANY ($1::VARCHAR[]))
GROUP BY
    f.call_sign
HAVING
    COUNT(*) = 1
ON CONFLICT DO NOTHING;
            "#,
            &self.ignored_conflict_call_signs
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
INSERT INTO
    fiskeridir_ais_vessel_mapping_whitelist (fiskeridir_vessel_id)
SELECT
    f.fiskeridir_vessel_id
FROM
    fiskeridir_vessels AS f
WHERE
    f.call_sign IS NULL
    OR f.call_sign = ANY ($1::VARCHAR[])
ON CONFLICT DO NOTHING
            "#,
            &self.ignored_conflict_call_signs
        )
        .execute(&mut **tx)
        .await?;

        let conflicts = sqlx::query!(
            r#"
SELECT
    ARRAY_AGG(DISTINCT f.fiskeridir_vessel_id) AS "fiskeridir_vessel_ids!: Vec<Option<FiskeridirVesselId>>",
    f.call_sign AS "call_sign!: CallSign",
    COALESCE(ARRAY_AGG(DISTINCT a.mmsi), '{}') AS "mmsis!: Vec<Option<Mmsi>>",
    COALESCE(ARRAY_AGG(DISTINCT a.name), '{}') AS "ais_vessel_names!: Vec<Option<String>>",
    COALESCE(ARRAY_AGG(DISTINCT f.name), '{}') AS "fiskeridir_vessel_names!: Vec<Option<String>>",
    COALESCE(
        ARRAY_AGG(DISTINCT f.fiskeridir_vessel_source_id),
        '{}'
    ) AS "fiskeridir_vessel_source_ids!: Vec<Option<VesselSource>>"
FROM
    fiskeridir_vessels AS f
    LEFT JOIN fiskeridir_ais_vessel_mapping_whitelist w ON f.fiskeridir_vessel_id = w.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
WHERE
    (
        w.is_manual = FALSE
        OR w.is_manual IS NULL
    )
    AND f.call_sign IS NOT NULL
    AND NOT (f.call_sign = ANY ($1::VARCHAR[]))
GROUP BY
    f.call_sign
HAVING
    COUNT(*) > 1
            "#,
            &self.ignored_conflict_call_signs
        )
        .fetch_all(&mut **tx)
        .await?;

        for c in &conflicts {
            sqlx::query!(
                r#"
INSERT INTO
    fiskeridir_ais_vessel_active_conflicts (
        call_sign,
        mmsis,
        fiskeridir_vessel_ids,
        ais_vessel_names,
        fiskeridir_vessel_names,
        fiskeridir_vessel_source_ids
    )
VALUES
    ($1, $2, $3, $4, $5, $6)
                "#,
                c.call_sign.as_ref(),
                &c.mmsis as &[Option<Mmsi>],
                &c.fiskeridir_vessel_ids as &[Option<FiskeridirVesselId>],
                &c.ais_vessel_names as &[Option<String>],
                &c.fiskeridir_vessel_names as &[Option<String>],
                &c.fiskeridir_vessel_source_ids as &[Option<VesselSource>],
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn set_landing_vessels_call_signs<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE fiskeridir_vessels
SET
    call_sign = q.call_sign,
    overriden_call_sign = fiskeridir_vessels.call_sign,
    call_sign_override = TRUE
FROM
    (
        SELECT
            qi.fiskeridir_vessel_id,
            (
                ARRAY_AGG(
                    vessel_call_sign
                    ORDER BY
                        qi.landing_ids DESC
                )
            ) [1] AS call_sign
        FROM
            (
                SELECT
                    fiskeridir_vessel_id,
                    vessel_call_sign,
                    COUNT(landing_id) AS landing_ids
                FROM
                    landings
                WHERE
                    vessel_call_sign IS NOT NULL
                    AND NOT (vessel_call_sign = ANY ($2::VARCHAR[]))
                GROUP BY
                    fiskeridir_vessel_id,
                    vessel_call_sign
            ) qi
        GROUP BY
            qi.fiskeridir_vessel_id
        HAVING
            COUNT(DISTINCT vessel_call_sign) > 1
    ) q
WHERE
    fiskeridir_vessels.fiskeridir_vessel_id = q.fiskeridir_vessel_id
    AND fiskeridir_vessel_source_id = $1;
            "#,
            VesselSource::Landings as i32,
            &self.ignored_conflict_call_signs
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn add_register_vessels_full(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> Result<()> {
        let municipalitis: HashMap<i32, NewMunicipality<'_>> = vessels
            .iter()
            .map(|v| {
                (
                    v.municipality_code,
                    NewMunicipality {
                        id: v.municipality_code,
                        name: None,
                    },
                )
            })
            .collect();

        let mut tx = self.pool.begin().await?;

        self.unnest_insert(municipalitis.into_values(), &mut *tx)
            .await?;
        self.unnest_insert_try_from::<_, _, NewRegisterVessel>(vessels, &mut *tx)
            .await?;

        self.set_landing_vessels_call_signs(&mut tx).await?;
        self.refresh_vessel_mappings(&mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations(
        &self,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination>> + '_ {
        self.fiskeridir_ais_vessel_combinations_impl(None)
    }

    pub(crate) async fn single_fiskeridir_ais_vessel_combination(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Option<FiskeridirAisVesselCombination>> {
        self.fiskeridir_ais_vessel_combinations_impl(Some(vessel_id))
            .next()
            .await
            .transpose()
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations_impl(
        &self,
        vessel_id: Option<FiskeridirVesselId>,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination>> + '_ {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.preferred_trip_assembler AS "preferred_trip_assembler!: TripAssemblerId",
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    f.fiskeridir_vessel_type_id,
    f.fiskeridir_length_group_id AS "fiskeridir_length_group_id!: VesselLengthGroup",
    f.fiskeridir_nation_group_id,
    f.norwegian_municipality_id AS fiskeridir_norwegian_municipality_id,
    f.norwegian_county_id AS fiskeridir_norwegian_county_id,
    f.nation_id AS "fiskeridir_nation_id?",
    f.gross_tonnage_1969 AS fiskeridir_gross_tonnage_1969,
    f.gross_tonnage_other AS fiskeridir_gross_tonnage_other,
    MAX(v.call_sign) AS "fiskeridir_call_sign: CallSign",
    f."name" AS fiskeridir_name,
    f.registration_id AS fiskeridir_registration_id,
    f."length" AS fiskeridir_length,
    f."width" AS fiskeridir_width,
    f."owner" AS fiskeridir_owner,
    f.owners::TEXT AS fiskeridir_owners,
    f.engine_building_year AS fiskeridir_engine_building_year,
    f.engine_power AS fiskeridir_engine_power,
    f.building_year AS fiskeridir_building_year,
    f.rebuilding_year AS fiskeridir_rebuilding_year,
    f.gear_group_ids AS "gear_group_ids!: Vec<GearGroup>",
    f.species_group_ids AS "species_group_ids!: Vec<SpeciesGroup>",
    a.mmsi AS "ais_mmsi?: Mmsi",
    a.imo_number AS ais_imo_number,
    a.call_sign AS "ais_call_sign: CallSign",
    a.name AS ais_name,
    a.ship_length AS ais_ship_length,
    a.ship_width AS ais_ship_width,
    a.eta AS ais_eta,
    a.destination AS ais_destination
FROM
    fiskeridir_ais_vessel_mapping_whitelist AS v
    INNER JOIN fiskeridir_vessels AS f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON v.mmsi = a.mmsi
WHERE
    (
        $1::BIGINT IS NULL
        OR f.fiskeridir_vessel_id = $1
    )
GROUP BY
    f.fiskeridir_vessel_id,
    a.mmsi
            "#,
            vessel_id as Option<FiskeridirVesselId>,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn add_vessel_gear_and_species_groups<'a>(
        &'a self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
UPDATE fiskeridir_vessels v
SET
    gear_group_ids = q.gear_group_ids,
    species_group_ids = q.species_group_ids
FROM
    (
        SELECT
            fiskeridir_vessel_id,
            ARRAY_AGG(DISTINCT gear_group_id) AS gear_group_ids,
            ARRAY_AGG(DISTINCT species_group_id) AS species_group_ids
        FROM
            landings l
            LEFT JOIN landing_entries e ON l.landing_id = e.landing_id
        WHERE
            fiskeridir_vessel_id IS NOT NULL
        GROUP BY
            fiskeridir_vessel_id
    ) q
WHERE
    v.fiskeridir_vessel_id = q.fiskeridir_vessel_id
            "#,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub(crate) async fn update_preferred_trip_assemblers_impl(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let vessel_ids = sqlx::query!(
            r#"
UPDATE fiskeridir_vessels f
SET
    preferred_trip_assembler = $1
FROM
    (
        SELECT
            v.fiskeridir_vessel_id
        FROM
            fiskeridir_vessels v
            INNER JOIN landings l ON v.fiskeridir_vessel_id = l.fiskeridir_vessel_id
            INNER JOIN ers_arrivals e ON v.fiskeridir_vessel_id = e.fiskeridir_vessel_id
        GROUP BY
            v.fiskeridir_vessel_id
        HAVING
            MAX(l.landing_timestamp) - MAX(e.arrival_timestamp) > INTERVAL '1 year'
    ) q
WHERE
    f.fiskeridir_vessel_id = q.fiskeridir_vessel_id
RETURNING
    f.fiskeridir_vessel_id AS "id!: FiskeridirVesselId"
            "#,
            TripAssemblerId::Landings as i32,
        )
        .fetch(&mut *tx)
        .map_ok(|r| r.id)
        .try_collect::<Vec<_>>()
        .await?;

        sqlx::query!(
            r#"
UPDATE fiskeridir_vessels f
SET
    preferred_trip_assembler = $1
FROM
    (
        SELECT DISTINCT
            v.fiskeridir_vessel_id
        FROM
            fiskeridir_vessels v
            INNER JOIN ers_departures e ON v.fiskeridir_vessel_id = e.fiskeridir_vessel_id
    ) q
WHERE
    f.fiskeridir_vessel_id = q.fiskeridir_vessel_id
    AND NOT (f.fiskeridir_vessel_id = ANY ($2::BIGINT[]))
            "#,
            TripAssemblerId::Ers as i32,
            &vessel_ids as &[FiskeridirVesselId],
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}
