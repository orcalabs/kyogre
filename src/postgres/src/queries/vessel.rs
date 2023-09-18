use std::collections::HashMap;

use crate::{
    error::PostgresError,
    models::{
        FiskeridirAisVesselCombination, NewFiskeridirVessel, NewMunicipality, NewRegisterVessel,
    },
    PostgresAdapter,
};
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::{Stream, TryStreamExt};
use kyogre_core::{FiskeridirVesselId, TripAssemblerId};
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn add_fiskeridir_vessels<'a>(
        &'a self,
        vessels: Vec<fiskeridir_rs::Vessel>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
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
            .collect::<Result<Vec<_>, _>>()?;

        NewFiskeridirVessel::unnest_insert(vessels, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) async fn add_register_vessels_full(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> Result<(), PostgresError> {
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

        let mut tx = self.begin().await?;

        self.add_municipalities(municipalitis.into_values().collect(), &mut tx)
            .await?;

        self.add_register_vessels_impl(vessels, &mut tx).await?;

        tx.commit()
            .await
            .change_context(PostgresError::Transaction)?;

        Ok(())
    }

    pub(crate) async fn add_register_vessels_impl<'a>(
        &'a self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        let vessels = vessels
            .into_iter()
            .map(NewRegisterVessel::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        NewRegisterVessel::unnest_insert(vessels, &mut **tx)
            .await
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub(crate) fn fiskeridir_ais_vessel_combinations(
        &self,
    ) -> impl Stream<Item = Result<FiskeridirAisVesselCombination, PostgresError>> + '_ {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.preferred_trip_assembler AS "preferred_trip_assembler!: TripAssemblerId",
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
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
    a.mmsi AS "ais_mmsi?",
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
    fiskeridir_vessels AS f
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
    LEFT JOIN vessel_benchmark_outputs AS b ON b.fiskeridir_vessel_id = f.fiskeridir_vessel_id
GROUP BY
    f.fiskeridir_vessel_id,
    a.mmsi
            "#
        )
        .fetch(&self.pool)
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn single_fiskeridir_ais_vessel_combination(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Option<FiskeridirAisVesselCombination>, PostgresError> {
        sqlx::query_as!(
            FiskeridirAisVesselCombination,
            r#"
SELECT
    f.preferred_trip_assembler AS "preferred_trip_assembler!: TripAssemblerId",
    f.fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
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
    a.mmsi AS "ais_mmsi?",
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
    fiskeridir_vessels AS f
    LEFT JOIN ais_vessels AS a ON f.call_sign = a.call_sign
    LEFT JOIN vessel_benchmark_outputs AS b ON b.fiskeridir_vessel_id = f.fiskeridir_vessel_id
WHERE
    f.fiskeridir_vessel_id = $1
GROUP BY
    f.fiskeridir_vessel_id,
    a.mmsi
            "#,
            vessel_id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| report!(e).change_context(PostgresError::Query))
    }

    pub(crate) async fn add_vessel_gear_and_species_groups<'a>(
        &'a self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
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
        .await
        .change_context(PostgresError::Query)
        .map(|_| ())
    }
}
