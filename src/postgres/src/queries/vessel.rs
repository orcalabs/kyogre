use std::collections::HashMap;

use crate::{
    error::Result,
    models::{
        ActiveVesselConflict, FiskeridirAisVesselCombination, NewFiskeridirVessel, NewMunicipality,
        NewRegisterVessel, VesselConflictInsert,
    },
    PostgresAdapter,
};
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::{FiskeridirVesselId, Mmsi, TripAssemblerId, VesselSource};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn active_vessel_conflicts_impl(&self) -> Result<Vec<ActiveVesselConflict>> {
        let conflicts = sqlx::query_as!(
            ActiveVesselConflict,
            r#"
SELECT
    call_sign,
    mmsis AS "mmsis!: Vec<Option<Mmsi>>",
    fiskeridir_vessel_ids AS "fiskeridir_vessel_ids!: Vec<Option<FiskeridirVesselId>>",
    ais_vessel_names AS "ais_vessel_names!: Vec<Option<String>>",
    fiskeridir_vessel_names AS "fiskeridir_vessel_names!: Vec<Option<String>>",
    fiskeridir_vessel_source_ids AS "fiskeridir_vessel_source_ids!: Vec<Option<VesselSource>>"
FROM
    fiskeridir_ais_vessel_active_conflicts
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(conflicts)
    }

    pub(crate) async fn manual_conflict_override_impl(
        &self,
        overrides: Vec<kyogre_core::NewVesselConflict>,
    ) -> Result<()> {
        let mut mmsi = Vec::with_capacity(overrides.len());
        let mut fiskeridir_vessel_id = Vec::with_capacity(overrides.len());

        let mut tx = self.pool.begin().await?;

        overrides.iter().for_each(|v| {
            if let Some(val) = v.mmsi {
                mmsi.push(val);
            }
            fiskeridir_vessel_id.push(v.vessel_id);
        });
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
        .fetch_all(&mut *tx)
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
        .fetch_all(&mut *tx)
        .await?;

        let overrides: Vec<VesselConflictInsert> = overrides
            .into_iter()
            .map(VesselConflictInsert::from)
            .collect();

        VesselConflictInsert::unnest_insert(overrides, &mut *tx).await?;

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

        let conflicts = sqlx::query_as!(
            ActiveVesselConflict,
            r#"
SELECT
    ARRAY_AGG(DISTINCT f.fiskeridir_vessel_id) AS "fiskeridir_vessel_ids!: Vec<Option<FiskeridirVesselId>>",
    f.call_sign AS "call_sign!",
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
        .await
        ?;

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
                c.call_sign,
                &c.mmsis as _,
                &c.fiskeridir_vessel_ids as _,
                &c.ais_vessel_names as _,
                &c.fiskeridir_vessel_names as _,
                &c.fiskeridir_vessel_source_ids
                    .iter()
                    .map(|i| i.map(|v| v as i32))
                    .collect::<Vec<_>>() as _,
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }
    pub(crate) async fn add_fiskeridir_vessels<'a>(
        &'a self,
        vessels: Vec<fiskeridir_rs::Vessel>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let vessels = vessels
            .into_iter()
            .map(NewFiskeridirVessel::try_from)
            .filter(|v| {
                matches!(
                    v,
                    Ok(NewFiskeridirVessel {
                        fiskeridir_vessel_id: Some(_),
                        ..
                    })
                )
            })
            .collect::<Result<Vec<_>>>()?;

        NewFiskeridirVessel::unnest_insert(vessels, &mut **tx).await?;

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
        let municipalitis: HashMap<i32, NewMunicipality> = vessels
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

        self.add_municipalities(municipalitis.into_values().collect(), &mut tx)
            .await?;

        self.add_register_vessels_impl(vessels, &mut tx).await?;
        self.set_landing_vessels_call_signs(&mut tx).await?;
        self.refresh_vessel_mappings(&mut tx).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn add_register_vessels_impl<'a>(
        &'a self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<()> {
        let vessels = vessels
            .into_iter()
            .map(NewRegisterVessel::try_from)
            .collect::<Result<Vec<_>>>()?;

        NewRegisterVessel::unnest_insert(vessels, &mut **tx).await?;

        Ok(())
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations(
        &self,
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
    MAX(v.call_sign) AS fiskeridir_call_sign,
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
    a.mmsi AS "ais_mmsi: Mmsi",
    a.imo_number AS ais_imo_number,
    a.call_sign AS ais_call_sign,
    a.name AS ais_name,
    a.ship_length AS ais_ship_length,
    a.ship_width AS ais_ship_width,
    a.eta AS ais_eta,
    a.destination AS ais_destination,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'benchmark_id',
                b.vessel_benchmark_id,
                'value',
                b.output
            )
        ) FILTER (
            WHERE
                b.output IS NOT NULL
                AND b.vessel_benchmark_id IS NOT NULL
        ),
        '[]'::jsonb
    )::TEXT AS "benchmarks!"
FROM
    fiskeridir_ais_vessel_mapping_whitelist AS v
    INNER JOIN fiskeridir_vessels AS f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON v.mmsi = a.mmsi
    LEFT JOIN vessel_benchmark_outputs AS b ON b.fiskeridir_vessel_id = v.fiskeridir_vessel_id
GROUP BY
    f.fiskeridir_vessel_id,
    a.mmsi
            "#
        )
        .fetch(&self.pool)
        .map_err(From::from)
    }

    pub(crate) async fn single_fiskeridir_ais_vessel_combination(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Option<FiskeridirAisVesselCombination>> {
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
    f.call_sign AS fiskeridir_call_sign,
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
    a.mmsi AS "ais_mmsi: Mmsi",
    a.imo_number AS ais_imo_number,
    a.call_sign AS ais_call_sign,
    a.name AS ais_name,
    a.ship_length AS ais_ship_length,
    a.ship_width AS ais_ship_width,
    a.eta AS ais_eta,
    a.destination AS ais_destination,
    COALESCE(
        JSONB_AGG(
            JSONB_BUILD_OBJECT(
                'benchmark_id',
                b.vessel_benchmark_id,
                'value',
                b.output
            )
        ) FILTER (
            WHERE
                b.output IS NOT NULL
                AND b.vessel_benchmark_id IS NOT NULL
        ),
        '[]'::jsonb
    )::TEXT AS "benchmarks!"
FROM
    fiskeridir_ais_vessel_mapping_whitelist AS v
    INNER JOIN fiskeridir_vessels AS f ON v.fiskeridir_vessel_id = f.fiskeridir_vessel_id
    LEFT JOIN ais_vessels AS a ON v.mmsi = a.mmsi
    LEFT JOIN vessel_benchmark_outputs AS b ON b.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    f.fiskeridir_vessel_id = $1
GROUP BY
    f.fiskeridir_vessel_id,
    a.mmsi
            "#,
            vessel_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(From::from)
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
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>();

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
